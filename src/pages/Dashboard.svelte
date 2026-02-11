<script lang="ts">
  import { currentView, terminalOpen, hasCompletedOnboarding } from "../stores/app";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import SessionTrace from "../components/SessionTrace.svelte";
  import ShareProof from "../components/ShareProof.svelte";

  interface LogEntry { ts: string; kind: string; msg: string; }
  interface WalletInfo { has_wallet: boolean; address: string; balance_cents: number; network: string; }
  interface VaultEntryInfo { alias: string; provider: string; preview: string; created_at: string; }
  interface GatewayHealth {
    running: boolean; port: number; model: string; auth_mode: string;
    bind: string; config_secured: boolean; unsecured_keys: string[]; config_path: string;
  }
  interface SecureLaunchResult {
    success: boolean; keys_injected: number; daemon_restarted: boolean;
    env_cleaned: boolean; detail: string;
  }
  interface NewSecretFound { key_name: string; file: string; provider: string; preview: string; }

  let events: LogEntry[] = [];
  let wallet: WalletInfo | null = null;
  let proxyRunning = false;
  let vaultUnlocked = false;
  let vaultEntries: VaultEntryInfo[] = [];
  let gateway: GatewayHealth | null = null;
  let pollId: ReturnType<typeof setInterval> | null = null;
  let showShare = false;

  let launching = false;
  let launchResult: SecureLaunchResult | null = null;
  let launchError = "";
  let newSecrets: NewSecretFound[] = [];
  let pinging = false;
  let pingResult = "";
  let pingOk = false;
  let vaultHasSecrets = false;

  async function loadAll() {
    try {
      events = await invoke("get_evidence_log");
      proxyRunning = await invoke("get_proxy_status");
      wallet = await invoke("get_wallet_info");
      vaultUnlocked = await invoke("vault_is_unlocked");
      if (vaultUnlocked) {
        vaultEntries = await invoke("vault_list_entries");
        vaultHasSecrets = vaultEntries.length > 0;
      }
    } catch (_) {}
    try { gateway = await invoke("check_gateway_health"); } catch (_) { gateway = null; }
    try { newSecrets = await invoke("scan_for_new_secrets"); } catch (_) { newSecrets = []; }
  }

  onMount(() => {
    loadAll();
    pollId = setInterval(loadAll, 4000);
    return () => { if (pollId) clearInterval(pollId); };
  });

  async function startProxy() { try { await invoke("start_proxy"); proxyRunning = true; } catch (_) {} }
  async function stopProxy() { try { await invoke("stop_proxy"); proxyRunning = false; } catch (_) {} }
  async function emergencyStop() { try { await invoke("stop_proxy"); proxyRunning = false; } catch (_) {} }

  async function launchSecureAgent() {
    launching = true;
    launchError = "";
    launchResult = null;
    try {
      launchResult = await invoke<SecureLaunchResult>("launch_secure_agent");
      if (!launchResult.success) {
        launchError = launchResult.detail;
      }
      await loadAll();
    } catch (e) {
      launchError = String(e);
    }
    launching = false;
  }

  async function pingAgent() {
    pinging = true;
    pingResult = "";
    pingOk = false;
    try {
      // Use openclaw agent --message to send a quick test
      const result = await invoke<{ ready: boolean; source: string; diagnostics: string[] }>(
        "check_openclaw_readiness", { path: null }
      );
      if (result.ready) {
        pingOk = true;
        pingResult = "Agent is responding (verified via " + result.source + ")";
      } else {
        pingResult = "Agent not responding: " + result.diagnostics.join(", ");
      }
    } catch (e) {
      pingResult = "Ping failed: " + String(e);
    }
    pinging = false;
  }

  function openChatWithAgent() {
    terminalOpen.set(true);
    // Give the terminal a moment to mount, then send the command
    setTimeout(() => {
      // The terminal is managed by App.svelte, we dispatch a custom event
      window.dispatchEvent(new CustomEvent("vault0-terminal-command", { detail: "openclaw tui\n" }));
    }, 500);
  }

  function resetOpenClaw() {
    terminalOpen.set(true);
    setTimeout(() => {
      window.dispatchEvent(new CustomEvent("vault0-terminal-command", { detail: "openclaw reset\n" }));
    }, 500);
  }

  function uninstallOpenClaw() {
    terminalOpen.set(true);
    setTimeout(() => {
      window.dispatchEvent(new CustomEvent("vault0-terminal-command", { detail: "openclaw uninstall\n" }));
    }, 500);
  }

  function goToHarden() {
    hasCompletedOnboarding.set(false);
    currentView.set("welcome");
  }

  async function migrateNewSecret(keyName: string, file: string) {
    const home = "~/.openclaw";
    try {
      const keys: [string, string][] = [[keyName, keyName]];
      await invoke("secure_config_keys", { installPath: home, keysToSecure: keys });
      await loadAll();
    } catch (_) {}
  }

  $: allowedCount = events.filter(e => e.kind === "allowed").length;
  $: blockedCount = events.filter(e => e.kind === "blocked").length;
  $: paymentCount = events.filter(e => e.kind === "payment").length;
</script>

<div class="p-6 space-y-5 max-w-4xl mx-auto">
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-bold">Overview</h1>
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-1.5">
        <span class="h-2 w-2 rounded-full {gateway?.running ? 'bg-emerald-400' : 'bg-red-400'}"></span>
        <span class="text-xs {gateway?.running ? 'text-emerald-400' : 'text-red-400'}">Gateway</span>
      </div>
      <div class="flex items-center gap-1.5">
        <span class="h-2 w-2 rounded-full {proxyRunning ? 'bg-emerald-400' : 'bg-red-400'}"></span>
        <span class="text-xs {proxyRunning ? 'text-emerald-400' : 'text-red-400'}">Proxy</span>
      </div>
      <div class="flex items-center gap-1.5">
        <span class="h-2 w-2 rounded-full {vaultUnlocked ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
        <span class="text-xs {vaultUnlocked ? 'text-emerald-400' : 'text-amber-400'}">Vault</span>
      </div>
      {#if proxyRunning}
        <button class="rounded border border-zinc-600 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-800" on:click={stopProxy}>Stop Proxy</button>
      {:else}
        <button class="rounded bg-emerald-600 px-2 py-1 text-xs text-white hover:bg-emerald-500" on:click={startProxy}>Start Proxy</button>
      {/if}
    </div>
  </div>

  <!-- Harden Prompt (shown when vault has no secrets) -->
  {#if !vaultHasSecrets}
    <div class="rounded-xl border-2 border-amber-600 bg-amber-950/20 p-5 space-y-3">
      <div class="flex items-center gap-2">
        <span class="text-xl">⚠</span>
        <h2 class="text-base font-bold text-amber-300">Your Agent is NOT Yet Hardened</h2>
      </div>
      <p class="text-sm text-zinc-300">Your API keys are still stored as plaintext on disk. Anyone with access to your machine can read them.</p>
      <p class="text-sm text-zinc-400">Click "Harden Now" to encrypt your secrets in the Vault-0 vault, apply a security policy, and restart your agent securely.</p>
      <button
        type="button"
        class="w-full rounded-xl bg-amber-600 px-5 py-3 text-base font-bold text-white hover:bg-amber-500"
        on:click={goToHarden}
      >
        Harden Now
      </button>
    </div>
  {/if}

  <!-- Card 1: Is My Agent Running? -->
  <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-5 space-y-3">
    <div class="flex items-center gap-2">
      <span class="text-xl">{gateway?.running ? "✅" : "❌"}</span>
      <h2 class="text-base font-semibold">Is My Agent Running?</h2>
    </div>
    {#if gateway?.running}
      <p class="text-sm text-emerald-400">Your OpenClaw agent is active and secured.</p>
      <div class="grid grid-cols-2 gap-3 text-sm">
        <div><span class="text-xs text-zinc-500">Model</span><p class="font-mono text-zinc-200">{gateway.model}</p></div>
        <div><span class="text-xs text-zinc-500">Gateway</span><p class="font-mono text-zinc-200">127.0.0.1:{gateway.port}</p></div>
        <div><span class="text-xs text-zinc-500">Auth</span><p class="text-zinc-200">{gateway.auth_mode}</p></div>
        <div><span class="text-xs text-zinc-500">Config</span>
          <p class={gateway.config_secured ? "text-emerald-400 text-xs" : "text-red-400 text-xs"}>
            {gateway.config_secured ? "Secured with vault tokens" : "Plaintext keys detected"}
          </p>
        </div>
      </div>
    {:else}
      <p class="text-sm text-red-400">Your OpenClaw gateway is not responding.</p>
      <p class="text-xs text-zinc-500">Start your gateway: open terminal below and run <span class="font-mono">openclaw gateway</span></p>
    {/if}

    <div class="border-t border-zinc-800 pt-3 space-y-2">
      <div class="grid grid-cols-2 gap-2">
        <button
          class="rounded-lg bg-emerald-600 px-4 py-3 text-sm font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
          disabled={launching || !vaultUnlocked}
          on:click={launchSecureAgent}
        >
          {launching ? "Restarting..." : "Restart Secure Agent"}
        </button>
        <button
          class="rounded-lg border border-emerald-600 px-4 py-3 text-sm font-semibold text-emerald-300 hover:bg-emerald-950/40"
          on:click={openChatWithAgent}
        >
          Chat with Agent
        </button>
      </div>
      <div class="flex gap-2">
        <button
          class="rounded border border-zinc-600 px-3 py-1.5 text-xs text-zinc-400 hover:text-white hover:bg-zinc-800 disabled:opacity-50"
          disabled={pinging}
          on:click={pingAgent}
        >
          {pinging ? "Pinging..." : "Ping Agent"}
        </button>
        {#if pingResult}
          <span class="text-xs {pingOk ? 'text-emerald-400' : 'text-red-400'}">{pingResult}</span>
        {/if}
      </div>
      <p class="text-xs text-zinc-500">Restart Secure Agent: re-injects secrets from vault into daemon memory. Use after crashes or updates.</p>
      {#if launchResult?.success}
        <p class="text-xs text-emerald-400">{launchResult.detail}</p>
      {/if}
      {#if launchError}
        <p class="text-xs text-red-400">{launchError}</p>
      {/if}
      {#if !vaultUnlocked}
        <p class="text-xs text-amber-400">Unlock your vault first to use secure launch.</p>
      {/if}
    </div>
  </div>

  <!-- Card 2: Are My Secrets Safe? -->
  <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-5 space-y-3">
    <div class="flex items-center gap-2">
      <span class="text-xl">{vaultUnlocked ? "🔓" : "🔒"}</span>
      <h2 class="text-base font-semibold">Are My Secrets Safe?</h2>
    </div>
    {#if vaultUnlocked}
      <p class="text-sm text-emerald-400">{vaultEntries.length} secret{vaultEntries.length !== 1 ? "s" : ""} encrypted in vault.</p>
      {#if vaultEntries.length > 0}
        <div class="space-y-1">
          {#each vaultEntries as entry}
            <div class="flex items-center justify-between rounded-lg bg-zinc-800/50 px-3 py-2">
              <div class="flex items-center gap-2">
                <span class="font-mono text-sm text-zinc-200">{entry.alias}</span>
                <span class="rounded bg-zinc-700 px-1.5 py-0.5 text-[10px] text-zinc-400">{entry.provider}</span>
              </div>
              <span class="font-mono text-xs text-zinc-600">{entry.preview}</span>
            </div>
          {/each}
        </div>
      {/if}
      <button class="text-xs text-emerald-400 hover:text-emerald-300" on:click={() => currentView.set("secrets")}>Manage secrets →</button>

      {#if newSecrets.length > 0}
        <div class="border-t border-zinc-800 pt-3 space-y-2">
          <div class="flex items-center gap-2">
            <span class="text-amber-400">⚠</span>
            <p class="text-sm text-amber-300">{newSecrets.length} new plaintext secret{newSecrets.length > 1 ? "s" : ""} detected on disk</p>
          </div>
          {#each newSecrets as ns}
            <div class="flex items-center justify-between rounded-lg bg-amber-950/20 border border-amber-800/40 px-3 py-2">
              <div class="flex items-center gap-2">
                <span class="font-mono text-xs text-amber-300">{ns.key_name}</span>
                <span class="text-xs text-zinc-500">in {ns.file}</span>
                <span class="font-mono text-xs text-zinc-600">{ns.preview}</span>
              </div>
            </div>
          {/each}
          <p class="text-xs text-zinc-500">These keys were added by OpenClaw and are not yet in your vault. Go to Secrets to migrate them.</p>
          <button class="text-xs text-amber-400 hover:text-amber-300" on:click={() => currentView.set("secrets")}>Migrate to vault →</button>
        </div>
      {/if}
    {:else}
      <p class="text-sm text-amber-400">Vault is locked. Unlock it to view and use your secrets.</p>
    {/if}
  </div>

  <!-- Card 3: What Happened Recently? -->
  <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-5 space-y-3">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <span class="text-xl">📡</span>
        <h2 class="text-base font-semibold">What Happened Recently?</h2>
      </div>
      <div class="flex items-center gap-3 text-xs text-zinc-500">
        <span>{allowedCount} proxied</span>
        <span>{blockedCount} blocked</span>
        <span>{paymentCount} payments</span>
        <button class="rounded border border-zinc-600 px-2 py-0.5 text-zinc-400 hover:text-white" on:click={() => (showShare = !showShare)}>
          {showShare ? "Hide" : "Share Proof"}
        </button>
      </div>
    </div>
    <ShareProof {events} walletAddress={wallet?.address || ""} visible={showShare} />
    {#if events.length === 0}
      <div class="rounded-lg bg-zinc-800/40 p-4 text-center">
        <p class="text-sm text-zinc-400">No activity yet.</p>
        <p class="text-xs text-zinc-500 mt-1">When your agent makes API calls through the Vault-0 proxy, they appear here.</p>
      </div>
    {:else}
      <SessionTrace {events} />
    {/if}
  </div>

  <!-- Card 4: Quick Actions + Emergency -->
  <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-5 space-y-4">
    <div class="flex items-center gap-2">
      <span class="text-xl">⚡</span>
      <h2 class="text-base font-semibold">Quick Actions</h2>
    </div>
    <div class="grid grid-cols-2 gap-2">
      <button class="rounded-lg border border-zinc-700 px-4 py-3 text-sm text-zinc-300 hover:bg-zinc-800 text-left" on:click={() => currentView.set("secrets")}>
        🔑 Manage Secrets
      </button>
      <button class="rounded-lg border border-zinc-700 px-4 py-3 text-sm text-zinc-300 hover:bg-zinc-800 text-left" on:click={() => currentView.set("policies")}>
        🛡 Edit Policies
      </button>
      <button class="rounded-lg border border-zinc-700 px-4 py-3 text-sm text-zinc-300 hover:bg-zinc-800 text-left" on:click={() => currentView.set("evidence")}>
        📋 Activity Log
      </button>
      <button class="rounded-lg border border-zinc-700 px-4 py-3 text-sm text-zinc-300 hover:bg-zinc-800 text-left" on:click={() => currentView.set("payments")}>
        💰 Wallet
      </button>
    </div>

    <details class="rounded-lg border border-zinc-800 bg-zinc-800/30 p-3">
      <summary class="cursor-pointer text-sm text-zinc-400">How It Works</summary>
      <div class="mt-2 text-xs text-zinc-500 space-y-1">
        <p>1. Your agent calls API providers as normal.</p>
        <p>2. Requests route through the Vault-0 proxy (127.0.0.1:3840).</p>
        <p>3. The proxy reads the real API key from the encrypted vault.</p>
        <p>4. The key is injected into the request header in memory.</p>
        <p>5. The request is forwarded. The key never touches disk.</p>
      </div>
    </details>

    <div class="border-t border-zinc-800 pt-3 space-y-2">
      <p class="text-xs text-red-400 font-medium">Emergency</p>
      <div class="flex gap-2">
        <button class="flex-1 rounded bg-red-600 px-3 py-2 text-sm font-semibold text-white hover:bg-red-500" on:click={emergencyStop}>
          Emergency Stop
        </button>
      </div>
    </div>

    <details class="border-t border-zinc-800 pt-3">
      <summary class="cursor-pointer text-xs text-zinc-500 hover:text-zinc-400">Advanced: Reset or Uninstall OpenClaw</summary>
      <div class="mt-3 space-y-2">
        <p class="text-xs text-zinc-500">These commands use OpenClaw's built-in tools. They run in the terminal below.</p>
        <button
          class="w-full rounded border border-amber-800 px-3 py-2 text-xs text-amber-400 hover:bg-amber-950/30 text-left"
          on:click={resetOpenClaw}
        >
          Reset OpenClaw — clears config and restarts fresh
        </button>
        <button
          class="w-full rounded border border-red-800 px-3 py-2 text-xs text-red-400 hover:bg-red-950/30 text-left"
          on:click={uninstallOpenClaw}
        >
          Uninstall OpenClaw — fully removes from system
        </button>
      </div>
    </details>
  </div>
</div>
