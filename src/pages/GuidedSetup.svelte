<script lang="ts">
  import { currentView, hasCompletedOnboarding } from "../stores/app";
  import Terminal from "../components/Terminal.svelte";

  let terminalRef: Terminal | undefined;
  let cliInstalled = false;
  let wizardStarted = false;
  let wizardLikelyDone = false;

  function installCli() {
    if (!terminalRef) return;
    cliInstalled = true;
    terminalRef.write("npm install -g openclaw@latest && openclaw --version\n");
  }

  function runWizard() {
    if (!terminalRef) return;
    wizardStarted = true;
    terminalRef.write("openclaw onboard --install-daemon\n");
  }

  function onTerminalOutput(text: string) {
    if (wizardStarted && !wizardLikelyDone) {
      const lower = text.toLowerCase();
      if (lower.includes("hatch") || lower.includes("tui") || lower.includes("gateway service installed") || lower.includes("control ui")) {
        wizardLikelyDone = true;
      }
    }
  }

  function finish() {
    hasCompletedOnboarding.set(true);
    currentView.set("dashboard");
  }

  function goBack() {
    currentView.set("welcome");
  }
</script>

<div class="flex min-h-screen">
  <!-- LEFT: Helper Text -->
  <div class="w-[40%] flex flex-col justify-center border-r border-zinc-800 px-12 py-10">
    <div class="flex h-14 w-14 items-center justify-center rounded-2xl bg-emerald-600/20 text-2xl mb-6">📦</div>
    <h2 class="text-2xl font-bold mb-3">Install OpenClaw</h2>
    <p class="text-sm text-zinc-400 mb-6">Use the terminal on the right to install and set up OpenClaw. Follow the three steps below.</p>

    <div class="space-y-4 text-sm text-zinc-400">
      <div class="flex items-start gap-3">
        <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-emerald-600/20 text-xs font-bold text-emerald-400">1</span>
        <div>
          <p class="text-zinc-200 font-medium">Install the CLI</p>
          <p class="text-xs text-zinc-500 mt-0.5">Click "Install CLI" to install OpenClaw globally via npm. Requires Node.js 22+.</p>
        </div>
      </div>
      <div class="flex items-start gap-3">
        <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-emerald-600/20 text-xs font-bold text-emerald-400">2</span>
        <div>
          <p class="text-zinc-200 font-medium">Run the Setup Wizard</p>
          <p class="text-xs text-zinc-500 mt-0.5">Click "Run Wizard" to start OpenClaw's native onboarding. Follow its prompts to choose your AI provider, enter your API key, and configure channels.</p>
        </div>
      </div>
      <div class="flex items-start gap-3">
        <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-emerald-600/20 text-xs font-bold text-emerald-400">3</span>
        <div>
          <p class="text-zinc-200 font-medium">Finish + Secure</p>
          <p class="text-xs text-zinc-500 mt-0.5">When the wizard is done, click "Finish" to go to the Dashboard. You'll be prompted to harden your install there.</p>
        </div>
      </div>
    </div>

    <p class="mt-8 text-xs text-zinc-600">macOS only. All data stays on this device.</p>
  </div>

  <!-- RIGHT: Terminal + Action Buttons -->
  <div class="w-[60%] flex flex-col min-h-0">
    <div class="flex items-center gap-2 border-b border-zinc-800 px-4 py-3">
      <button type="button" class="text-sm text-zinc-400 hover:text-white" on:click={goBack}>Back</button>
      <span class="text-sm font-semibold">Terminal</span>
    </div>

    <div class="flex-1 min-h-0 p-4">
      <Terminal bind:this={terminalRef} onOutput={onTerminalOutput} logPrefix="InstallTerminal" />
    </div>

    <div class="flex items-center gap-2 border-t border-zinc-800 px-4 py-3">
      <button
        type="button"
        class="rounded-lg {cliInstalled ? 'bg-zinc-800 text-zinc-500' : 'bg-zinc-700 text-white hover:bg-zinc-600'} px-4 py-2.5 text-sm font-semibold"
        on:click={installCli}
      >
        {cliInstalled ? "✓ CLI Installed" : "1. Install CLI"}
      </button>
      <button
        type="button"
        class="rounded-lg {wizardStarted ? 'bg-zinc-800 text-zinc-500' : 'bg-zinc-700 text-white hover:bg-zinc-600'} px-4 py-2.5 text-sm font-semibold"
        on:click={runWizard}
      >
        {wizardStarted ? "✓ Wizard Running" : "2. Run Wizard"}
      </button>
      <button
        type="button"
        class="ml-auto rounded-lg px-6 py-2.5 text-sm font-semibold {wizardLikelyDone ? 'bg-emerald-600 text-white hover:bg-emerald-500' : 'bg-zinc-800 text-zinc-500 cursor-not-allowed'}"
        disabled={!wizardLikelyDone}
        on:click={finish}
      >
        3. Finish + Go to Dashboard
      </button>
      {#if wizardStarted && !wizardLikelyDone}
        <span class="text-xs text-zinc-500">Complete the wizard first...</span>
      {/if}
    </div>
  </div>
</div>
