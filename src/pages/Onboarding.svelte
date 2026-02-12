<script lang="ts">
  import { currentView, hasCompletedOnboarding, setupState } from "../stores/app";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface PlaintextKey { file: string; key_name: string; preview: string; }
  interface DetectionResult {
    found: boolean;
    path: string;
    install_kind: string;
    cli_version: string;
    has_config: boolean;
    plaintext_keys: PlaintextKey[];
  }
  interface HardenStep { step: string; status: string; detail: string; items: string[]; }
  interface HardenResult { success: boolean; steps: HardenStep[]; }
  interface SecureLaunchResult {
    success: boolean; keys_injected: number; daemon_restarted: boolean;
    env_cleaned: boolean; detail: string;
  }

  type PageState = "scanning" | "choose" | "found" | "not_found" | "passphrase" | "hardening" | "done";

  let state: PageState = "scanning";
  let detection: DetectionResult | null = null;
  let error = "";

  let passphrase = "";
  let passphraseConfirm = "";
  let passphraseError = "";
  let creatingVault = false;

  let hardenSteps: HardenStep[] = [];
  let hardenError = "";
  let hardenVisibleIndex = 0;
  let hardenRunning = false;

  let vaultExists = false;
  let vaultUnlocked = false;
  let unlockPassphrase = "";
  let unlockError = "";
  let unlocking = false;
  let showResetConfirm = false;
  let resetting = false;
  let showFullReset = false;
  let fullResetting = false;

  onMount(async () => {
    state = "scanning";
    try {
      vaultExists = await invoke<boolean>("vault_exists");
      if (vaultExists) {
        vaultUnlocked = await invoke<boolean>("vault_is_unlocked");
      }
      detection = await invoke<DetectionResult>("detect_openclaw");
      if (vaultExists && !vaultUnlocked) {
        state = "found";
      } else {
        state = "choose";
      }
    } catch (e: unknown) {
      detection = null;
      state = "not_found";
    }
  });

  function riskLevel(keys: PlaintextKey[]): string {
    if (keys.length >= 3) return "High";
    if (keys.length >= 1) return "Medium";
    return "Low";
  }

  function riskColor(level: string): string {
    if (level === "High") return "text-red-400 border-red-800 bg-red-950/30";
    if (level === "Medium") return "text-amber-400 border-amber-800 bg-amber-950/30";
    return "text-emerald-400 border-emerald-800 bg-emerald-950/30";
  }

  function passphraseStrength(p: string): { label: string; color: string; ok: boolean } {
    if (p.length < 12) return { label: "Too short (12+ required)", color: "text-red-400", ok: false };
    const hasUpper = /[A-Z]/.test(p);
    const hasLower = /[a-z]/.test(p);
    const hasDigit = /[0-9]/.test(p);
    const hasSymbol = /[^a-zA-Z0-9]/.test(p);
    const score = [hasUpper, hasLower, hasDigit, hasSymbol].filter(Boolean).length;
    if (score <= 2) return { label: "Weak", color: "text-amber-400", ok: true };
    if (score === 3) return { label: "Good", color: "text-emerald-400", ok: true };
    return { label: "Strong", color: "text-emerald-300", ok: true };
  }

  async function unlockVault() {
    unlockError = "";
    unlocking = true;
    try {
      await invoke("vault_unlock", { passphrase: unlockPassphrase });
      vaultUnlocked = true;
    } catch (e) {
      unlockError = String(e);
    }
    unlocking = false;
  }

  async function resetVault() {
    resetting = true;
    try {
      await invoke("vault_lock");
      await invoke("vault_delete_file");
      vaultExists = false;
      vaultUnlocked = false;
      showResetConfirm = false;
      unlockError = "";
      unlockPassphrase = "";
    } catch (_) {
      vaultExists = false;
      vaultUnlocked = false;
      showResetConfirm = false;
    }
    resetting = false;
  }

  function chooseSecure() {
    state = "found";
  }

  function chooseInstall() {
    goToWizard();
  }

  function startHarden() {
    if (vaultExists && vaultUnlocked) {
      state = "hardening";
      runHarden();
    } else if (vaultExists && !vaultUnlocked) {
      error = "Unlock your vault first.";
    } else {
      state = "passphrase";
    }
  }

  async function createVaultAndProceed() {
    passphraseError = "";
    const strength = passphraseStrength(passphrase);
    if (!strength.ok) {
      passphraseError = strength.label;
      return;
    }
    if (passphrase !== passphraseConfirm) {
      passphraseError = "Passphrases do not match.";
      return;
    }
    creatingVault = true;
    try {
      await invoke("vault_create", { passphrase });
      vaultExists = true;
      vaultUnlocked = true;
      state = "hardening";
      runHarden();
    } catch (e) {
      passphraseError = String(e);
    }
    creatingVault = false;
  }

  async function runHarden() {
    hardenSteps = [];
    hardenError = "";
    hardenVisibleIndex = 0;
    hardenRunning = true;
    try {
      const installPath = detection?.path || "";
      const result = await invoke<HardenResult>("harden_openclaw", { installPath });
      hardenSteps = result.steps;

      // Step 5: Auto-launch secure agent
      try {
        const launch = await invoke<SecureLaunchResult>("launch_secure_agent");
        const launchItems = [
          `${launch.keys_injected} keys written to .env temporarily`,
          launch.daemon_restarted ? "Daemon restarted successfully" : "Daemon restart failed (manual restart needed)",
          launch.env_cleaned ? ".env cleaned — no plaintext on disk" : ".env cleanup failed",
        ];
        hardenSteps = [...hardenSteps, {
          step: "launch",
          status: launch.success ? "ok" : "warn",
          detail: launch.success ? "Agent restarted with vault keys" : "Agent launch had issues",
          items: launchItems,
        }];
      } catch (e) {
        hardenSteps = [...hardenSteps, {
          step: "launch",
          status: "warn",
          detail: `Agent restart: ${String(e)}`,
          items: ["You can restart manually from the Dashboard using 'Restart Secure Agent'"],
        }];
      }

      hardenVisibleIndex = 1;
      hardenRunning = false;
      if (!result.success) {
        hardenError = "Hardening did not fully succeed. Check steps above.";
      }
    } catch (e) {
      hardenError = String(e);
      hardenRunning = false;
    }
  }

  function hardenNext() {
    if (hardenVisibleIndex < hardenSteps.length) {
      hardenVisibleIndex += 1;
    }
    if (hardenVisibleIndex >= hardenSteps.length) {
      state = "done";
    }
  }

  function goToDashboard() {
    hasCompletedOnboarding.set(true);
    currentView.set("dashboard");
  }

  function goToMonitor() {
    hasCompletedOnboarding.set(true);
    currentView.set("monitor");
  }

  function goToWizard() {
    setupState.set({ step: "setup", existingPath: "", useExisting: false });
    currentView.set("setup");
  }

  $: risk = detection?.plaintext_keys ? riskLevel(detection.plaintext_keys) : "Low";
  $: riskCss = riskColor(risk);
  $: strength = passphraseStrength(passphrase);
</script>

<div class="flex min-h-screen">
  <!-- LEFT: Context Panel -->
  <div class="w-[40%] flex flex-col justify-center border-r border-zinc-800 px-12 py-10">
    {#if state === "scanning"}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">🛡</div>
      <h2 class="text-2xl font-bold mb-3">Scanning Your System</h2>
      <p class="text-sm text-zinc-400 mb-4">Vault-0 is checking common OpenClaw install locations to find your agent configuration and detect any exposed secrets.</p>
      <div class="space-y-2 text-xs text-zinc-500">
        <p>Locations checked:</p>
        <p class="font-mono">~/.openclaw, ~/openclaw, ~/clawbot, ~/moltbot, global CLI PATH</p>
        <p>This usually takes less than 2 seconds.</p>
      </div>

    {:else if state === "choose"}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">V0</div>
      <h2 class="text-2xl font-bold mb-3">Welcome to Vault-0</h2>
      <p class="text-sm text-zinc-400 mb-4">Vault-0 is a secure command center for your OpenClaw AI agent. It encrypts your API keys, monitors agent activity, and gives you full control.</p>
      <div class="space-y-3 text-xs text-zinc-500 mt-4">
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">📡</span>
          <p>Monitor live agent activity: messages, tool calls, and thinking states in real time.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">🔒</span>
          <p>Encrypt all API keys and tokens in a local vault.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">🛡</span>
          <p>Apply security policies: domain allowlisting, spend caps, log redaction.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">🚨</span>
          <p>Emergency controls to stop your agent instantly.</p>
        </div>
      </div>
      {#if detection?.found}
        <div class="mt-6 rounded-lg border border-emerald-800 bg-emerald-950/20 px-3 py-2 text-xs text-emerald-400">
          OpenClaw detected at {detection.path}
        </div>
      {:else}
        <div class="mt-6 rounded-lg border border-zinc-700 bg-zinc-800/40 px-3 py-2 text-xs text-zinc-500">
          No existing OpenClaw installation found.
        </div>
      {/if}

    {:else if state === "found" && detection && vaultExists && !vaultUnlocked}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">🔒</div>
      <h2 class="text-2xl font-bold mb-3">Welcome Back</h2>
      <p class="text-sm text-zinc-400 mb-4">Your encrypted vault contains all the API keys and tokens Vault-0 previously secured. Enter your master passphrase to decrypt it.</p>
      <div class="space-y-3 text-xs text-zinc-500">
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">🔑</span>
          <p>The passphrase you created when you first set up Vault-0.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">💻</span>
          <p>It never leaves your machine. Vault-0 cannot recover it for you.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">🛡</span>
          <p>Encryption: Argon2id key derivation + AES-256-GCM.</p>
        </div>
      </div>

    {:else if state === "found" && detection}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">🔍</div>
      <h2 class="text-2xl font-bold mb-3">Security Scan Complete</h2>
      <p class="text-sm text-zinc-400 mb-4">We found your OpenClaw installation and scanned its config files for exposed secrets. Review the results and harden your setup with one click.</p>
      <div class="space-y-3 text-sm text-zinc-500 mt-4">
        <p class="text-xs font-medium text-zinc-400 uppercase tracking-wide">What "Harden Now" does</p>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">1.</span>
          <p><span class="text-zinc-300">Encrypted backup</span> of your current config files.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">2.</span>
          <p><span class="text-zinc-300">Migrate secrets</span> into an encrypted vault. Replace originals with safe alias tokens.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">3.</span>
          <p><span class="text-zinc-300">Security policy</span> with domain allowlisting, metadata blocking, and log redaction.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">4.</span>
          <p><span class="text-zinc-300">Start proxy</span> that injects real keys from the vault at runtime. Keys never touch disk.</p>
        </div>
        <p class="text-xs text-zinc-600 mt-2">All changes are reversible from the Dashboard.</p>
      </div>

    {:else if state === "not_found"}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-zinc-800 text-2xl mb-6">🔎</div>
      <h2 class="text-2xl font-bold mb-3">No Installation Found</h2>
      <p class="text-sm text-zinc-400 mb-4">Vault-0 scanned your system but could not find an OpenClaw installation. This means OpenClaw is either not installed or in a custom location.</p>
      <div class="space-y-2 text-xs text-zinc-500">
        <p>Locations checked:</p>
        <p class="font-mono">~/.openclaw, ~/openclaw, ~/clawbot, ~/moltbot, ~/Development/openclaw, ~/Code/openclaw, global PATH</p>
        <p class="mt-3">You can install OpenClaw using the guided wizard, or point Vault-0 to a custom path.</p>
      </div>

    {:else if state === "passphrase"}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">🔑</div>
      <h2 class="text-2xl font-bold mb-3">Your Master Passphrase</h2>
      <p class="text-sm text-zinc-400 mb-4">This passphrase encrypts all your agent secrets (API keys, tokens, credentials) into a local vault file. Without it, no one can read your secrets — not even Vault-0.</p>
      <div class="space-y-3 text-xs text-zinc-500 mt-2">
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">🔐</span>
          <p>Encryption: Argon2id + AES-256-GCM. Industry standard.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">💻</span>
          <p>Everything stays on your Mac. Nothing is sent anywhere.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">📋</span>
          <p>Tip: Use your Mac's Passwords app or 1Password/Bitwarden to generate and save this.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">⚠</span>
          <p>If you lose this passphrase, your vault cannot be recovered. Write it down or save it in a password manager.</p>
        </div>
      </div>

    {:else if state === "hardening"}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">🛡</div>
      <h2 class="text-2xl font-bold mb-3">Hardening In Progress</h2>
      <p class="text-sm text-zinc-400 mb-4">Vault-0 is securing your OpenClaw installation step by step. Review each step and click Next to continue.</p>
      {#if hardenSteps.length > 0 && hardenVisibleIndex > 0}
        {@const currentStep = hardenSteps[Math.min(hardenVisibleIndex - 1, hardenSteps.length - 1)]}
        <div class="rounded-lg border border-zinc-700 bg-zinc-800/40 p-3 mt-2 text-xs text-zinc-500">
          <p class="text-zinc-400 font-medium mb-1">About this step:</p>
          {#if currentStep.step === "backup"}
            <p>Your original config files are being copied and encrypted before any changes are made. This is your safety net — you can always restore from the Dashboard if needed.</p>
          {:else if currentStep.step === "migrate"}
            <p>Plaintext API keys are being moved from disk into the encrypted vault. The files on disk now only contain safe alias tokens (e.g., VAULT0_ALIAS:openai) that are meaningless without the vault.</p>
          {:else if currentStep.step === "policy"}
            <p>A security policy now controls which domains your agent can reach (e.g., api.openai.com), blocks access to cloud metadata endpoints, and redacts secrets from any logged output.</p>
          {:else if currentStep.step === "proxy"}
            <p>A local reverse proxy is running on your machine. When your agent makes an API call, the proxy reads the real key from the encrypted vault (in memory only) and injects it into the request header. The key is never written to disk.</p>
          {:else if currentStep.step === "launch"}
            <p>Your real API keys were briefly written to .env, the OpenClaw daemon was restarted to load them into memory, and then the .env file was immediately cleaned. Your agent is now running with real keys in memory but zero plaintext on disk.</p>
          {/if}
        </div>
      {/if}

    {:else if state === "done"}
      <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">✓</div>
      <h2 class="text-2xl font-bold text-emerald-300 mb-3">You're Protected</h2>
      <p class="text-sm text-zinc-400 mb-4">Your agent is running securely right now. API keys were encrypted, injected into the daemon's memory, and cleaned from disk. No plaintext secrets remain.</p>
      <div class="space-y-3 text-xs text-zinc-500 mt-4">
        <p class="text-zinc-400 font-medium">How it works now:</p>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">1.</span>
          <p>Your agent makes an API call (e.g., to api.openai.com).</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">2.</span>
          <p>Vault-0 decrypts your keys from the vault and writes them to .env temporarily.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">3.</span>
          <p>The OpenClaw daemon restarts and reads the keys into memory.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">4.</span>
          <p>Vault-0 zeros the .env file. Keys are on disk for about 2 seconds.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">5.</span>
          <p>Your agent runs with real keys in memory. No plaintext on disk.</p>
        </div>
      </div>
      <div class="space-y-3 text-xs text-zinc-500 mt-6">
        <p class="text-zinc-400 font-medium">On the Dashboard you can:</p>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">📡</span>
          <p>Monitor live agent activity, messages, and tool calls.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">🚨</span>
          <p>Emergency stop or revert all changes.</p>
        </div>
        <div class="flex items-start gap-2">
          <span class="text-emerald-400 mt-0.5">📤</span>
          <p>Share Proof that your agent is running securely.</p>
        </div>
      </div>
    {/if}

    <p class="mt-8 text-xs text-zinc-600">macOS only. All data stays on this device.</p>
  </div>

  <!-- RIGHT: Action Panel -->
  <div class="w-[60%] flex flex-col justify-center px-12 py-10 overflow-y-auto">
    <div class="max-w-lg mx-auto w-full space-y-5">
      <div class="flex items-center gap-3 mb-2">
        <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-emerald-600/20 text-xl font-bold text-emerald-400">V0</div>
        <h1 class="text-xl font-bold tracking-tight">Vault-0</h1>
      </div>

      {#if state === "scanning"}
        <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-8 text-center">
          <div class="animate-pulse text-3xl mb-3">🛡</div>
          <p class="text-base text-zinc-300">Scanning...</p>
        </div>

      {:else if state === "choose"}
        <div class="grid grid-cols-3 gap-3">
          <button
            type="button"
            class="rounded-xl border-2 p-5 text-left space-y-3 transition-colors {detection?.found ? 'border-emerald-600 bg-emerald-950/20 hover:bg-emerald-950/30' : 'border-zinc-700 bg-zinc-900/60 hover:bg-zinc-800/60'}"
            on:click={chooseSecure}
          >
            <div class="text-3xl">🛡</div>
            <h3 class="text-base font-bold">Secure My Agent</h3>
            <p class="text-xs text-zinc-400">Scan and harden my existing OpenClaw install.</p>
            {#if detection?.found}
              <span class="inline-block rounded bg-emerald-600/20 px-2 py-0.5 text-xs text-emerald-400">Recommended</span>
            {/if}
          </button>
          <button
            type="button"
            class="rounded-xl border-2 p-5 text-left space-y-3 transition-colors border-zinc-700 bg-zinc-900/60 hover:bg-zinc-800/60"
            on:click={goToMonitor}
          >
            <div class="text-3xl">📡</div>
            <h3 class="text-base font-bold">Just Monitor</h3>
            <p class="text-xs text-zinc-400">Skip setup. Watch my agent's live activity and security events.</p>
          </button>
          <button
            type="button"
            class="rounded-xl border-2 p-5 text-left space-y-3 transition-colors {!detection?.found ? 'border-emerald-600 bg-emerald-950/20 hover:bg-emerald-950/30' : 'border-zinc-700 bg-zinc-900/60 hover:bg-zinc-800/60'}"
            on:click={chooseInstall}
          >
            <div class="text-3xl">📦</div>
            <h3 class="text-base font-bold">Install OpenClaw</h3>
            <p class="text-xs text-zinc-400">Set up OpenClaw from scratch using the guided wizard.</p>
            {#if !detection?.found}
              <span class="inline-block rounded bg-emerald-600/20 px-2 py-0.5 text-xs text-emerald-400">Recommended</span>
            {/if}
          </button>
        </div>

      {:else if state === "found" && detection && vaultExists && !vaultUnlocked}
        <div class="space-y-4">
          <input
            type="password"
            id="vault-unlock-passphrase"
            name="password"
            autocomplete="current-password"
            bind:value={unlockPassphrase}
            placeholder="Master passphrase"
            class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-3 text-base text-white focus:border-emerald-500 focus:outline-none"
          />
          {#if unlockError}
            <p class="text-sm text-red-400">{unlockError}</p>
          {/if}
          <button
            type="button"
            class="w-full rounded-xl bg-emerald-600 px-5 py-3 text-base font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
            disabled={unlocking}
            on:click={unlockVault}
          >
            {unlocking ? "Unlocking..." : "Unlock Vault"}
          </button>
          {#if !showResetConfirm}
            <button
              type="button"
              class="w-full text-center text-xs text-zinc-500 hover:text-red-400"
              on:click={() => (showResetConfirm = true)}
            >
              Forgot passphrase? Reset vault...
            </button>
          {:else}
            <div class="rounded-lg border border-red-800 bg-red-950/30 p-3 space-y-2">
              <p class="text-sm text-red-300 font-medium">Are you sure?</p>
              <p class="text-xs text-red-400">This will permanently delete all stored secrets. You will need to re-enter your API keys and re-run hardening.</p>
              <div class="flex gap-2">
                <button type="button" class="flex-1 rounded-lg bg-red-600 px-3 py-2 text-sm font-semibold text-white hover:bg-red-500 disabled:opacity-50" disabled={resetting} on:click={resetVault}>
                  {resetting ? "Resetting..." : "Delete Vault + Reset"}
                </button>
                <button type="button" class="flex-1 rounded-lg border border-zinc-600 px-3 py-2 text-sm text-zinc-300 hover:bg-zinc-800" on:click={() => (showResetConfirm = false)}>
                  Cancel
                </button>
              </div>
            </div>
          {/if}
        </div>

      {:else if state === "found" && detection}
        <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-4 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-sm font-medium text-zinc-300">OpenClaw Detected</span>
            {#if detection.cli_version}
              <span class="text-xs text-zinc-500">v{detection.cli_version}</span>
            {/if}
          </div>
          <p class="font-mono text-sm text-emerald-400 break-all">{detection.path}</p>
          <p class="text-xs text-zinc-500">{detection.install_kind === "global_cli" ? "Global CLI" : detection.install_kind === "config_dir" ? "Config directory" : "Directory install"}</p>
        </div>

        <div class="rounded-xl border {riskCss} p-4 space-y-2">
          <div class="flex items-center gap-2">
            <span class="text-lg">{risk === "High" ? "⚠️" : risk === "Medium" ? "⚠️" : "✅"}</span>
            <h3 class="text-base font-semibold">{risk} Risk</h3>
          </div>
          {#if detection.plaintext_keys.length > 0}
            <p class="text-sm">{detection.plaintext_keys.length} plaintext secret{detection.plaintext_keys.length > 1 ? "s" : ""} found</p>
            <div class="rounded-lg bg-zinc-800/40 p-2 space-y-1">
              {#each detection.plaintext_keys as pk}
                <div class="flex items-center gap-2 text-xs">
                  <span class="font-mono text-zinc-300">{pk.key_name}</span>
                  <span class="text-zinc-600">{pk.file}</span>
                  <span class="font-mono text-zinc-600">{pk.preview}</span>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-sm">No plaintext secrets detected. Hardening still recommended.</p>
          {/if}
        </div>

        <button
          type="button"
          class="w-full rounded-xl bg-emerald-600 px-6 py-4 text-lg font-bold text-white shadow-lg shadow-emerald-600/20 transition hover:bg-emerald-500 active:scale-[0.98]"
          on:click={startHarden}
        >
          Harden Now
        </button>
        {#if error}
          <p class="text-sm text-red-400">{error}</p>
        {/if}

      {:else if state === "not_found"}
        <button
          type="button"
          class="w-full rounded-xl bg-emerald-600 px-6 py-4 text-lg font-semibold text-white hover:bg-emerald-500"
          on:click={goToWizard}
        >
          Install OpenClaw
        </button>

      {:else if state === "passphrase"}
        <div class="space-y-4">
          <div>
            <input
              type="password"
              id="vault-new-passphrase"
              name="new-password"
              autocomplete="new-password"
              bind:value={passphrase}
              placeholder="Master passphrase (12+ characters)"
              class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-3 text-base text-white focus:border-emerald-500 focus:outline-none"
            />
            {#if passphrase.length > 0}
              <p class="mt-1 text-sm {strength.color}">{strength.label}</p>
            {/if}
          </div>
          <input
            type="password"
            id="vault-confirm-passphrase"
            name="confirm-password"
            autocomplete="new-password"
            bind:value={passphraseConfirm}
            placeholder="Confirm passphrase"
            class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-3 text-base text-white focus:border-emerald-500 focus:outline-none"
          />
          {#if passphraseError}
            <p class="text-sm text-red-400">{passphraseError}</p>
          {/if}
          <button
            type="button"
            class="w-full rounded-xl bg-emerald-600 px-5 py-3 text-base font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
            disabled={creatingVault || !strength.ok}
            on:click={createVaultAndProceed}
          >
            {creatingVault ? "Creating Vault..." : "Create Vault + Harden"}
          </button>
        </div>

      {:else if state === "hardening"}
        <div class="space-y-4">
          {#if hardenRunning}
            <div class="flex items-center gap-2 text-sm text-zinc-400">
              <span class="animate-pulse">●</span>
              <span>Running security hardening...</span>
            </div>
          {/if}

          {#each hardenSteps.slice(0, hardenVisibleIndex) as step}
            <div class="rounded-lg border border-zinc-700 bg-zinc-800/50 p-4 space-y-2">
              <div class="flex items-center gap-2 text-sm">
                <span class={step.status === "ok" ? "text-emerald-400" : step.status === "warn" ? "text-amber-400" : "text-red-400"}>
                  {step.status === "ok" ? "✓" : step.status === "warn" ? "⚠" : "✗"}
                </span>
                <span class="font-medium text-zinc-200">
                  {step.step === "backup" ? "Backup Created" : ""}
                  {step.step === "migrate" ? "Secrets Migrated" : ""}
                  {step.step === "policy" ? "Policy Applied" : ""}
                  {step.step === "proxy" ? "Secure Proxy Started" : ""}
                  {step.step === "launch" ? "Agent Restarted with Vault Keys" : ""}
                </span>
              </div>
              {#if step.items && step.items.length > 0}
                <div class="pl-6 space-y-1">
                  {#each step.items as item}
                    <p class="font-mono text-xs text-zinc-500">{item}</p>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}

          {#if hardenError}
            <p class="text-sm text-red-400">{hardenError}</p>
          {/if}

          {#if !hardenRunning && hardenSteps.length > 0 && hardenVisibleIndex < hardenSteps.length}
            <button type="button" class="w-full rounded-xl bg-emerald-600 px-5 py-3 text-base font-semibold text-white hover:bg-emerald-500" on:click={hardenNext}>
              Next Step ({hardenVisibleIndex + 1} of {hardenSteps.length})
            </button>
          {/if}

          {#if !hardenRunning && hardenVisibleIndex >= hardenSteps.length && hardenSteps.length > 0}
            <button type="button" class="w-full rounded-xl bg-emerald-600 px-5 py-3 text-base font-semibold text-white hover:bg-emerald-500" on:click={hardenNext}>
              Complete
            </button>
          {/if}
        </div>

      {:else if state === "done"}
        <div class="space-y-5">
          <div class="grid grid-cols-2 gap-3">
            <div class="rounded-lg border border-red-800/60 bg-red-950/20 p-4 space-y-2">
              <p class="text-xs font-medium text-red-400 uppercase tracking-wide">Before</p>
              <div class="space-y-1.5 text-xs text-zinc-400">
                <p class="text-zinc-300">Your Agent → API Provider</p>
                <p class="font-mono text-red-400">Key: sk-proj-abc...xyz</p>
                <p class="text-red-400/70">PLAINTEXT ON DISK</p>
                <p>No policy enforcement</p>
                <p>No logging or redaction</p>
                <p>No spend controls</p>
              </div>
            </div>
            <div class="rounded-lg border border-emerald-800/60 bg-emerald-950/20 p-4 space-y-2">
              <p class="text-xs font-medium text-emerald-400 uppercase tracking-wide">After</p>
              <div class="space-y-1.5 text-xs text-zinc-400">
                <p class="text-zinc-300">Your Agent → Vault-0 → API Provider</p>
                <p class="font-mono text-emerald-400">Key: from encrypted vault</p>
                <p class="text-emerald-400/70">IN MEMORY ONLY</p>
                <p>Domain allowlist active</p>
                <p>Tamper-evident logging</p>
                <p>Spend cap: $10.00</p>
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-zinc-700 bg-zinc-800/40 p-4 space-y-2">
            <p class="text-xs font-medium text-zinc-400 uppercase tracking-wide">Changes Applied</p>
            {#each hardenSteps as step}
              <div class="space-y-1">
                <div class="flex items-center gap-2 text-sm">
                  <span class="text-emerald-400">✓</span>
                  <span class="text-zinc-300">{step.detail}</span>
                </div>
                {#if step.items && step.items.length > 0}
                  <div class="pl-6 space-y-0.5">
                    {#each step.items as item}
                      <p class="font-mono text-[11px] text-zinc-500">{item}</p>
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </div>

          <button
            type="button"
            class="w-full rounded-xl bg-emerald-600 px-6 py-4 text-lg font-bold text-white shadow-lg shadow-emerald-600/20 transition hover:bg-emerald-500 active:scale-[0.98]"
            on:click={goToDashboard}
          >
            Go to Dashboard
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>
