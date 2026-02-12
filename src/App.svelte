<script lang="ts">
  import Onboarding from "./pages/Onboarding.svelte";
  import GuidedSetup from "./pages/GuidedSetup.svelte";
  import Dashboard from "./pages/Dashboard.svelte";
  import Secrets from "./pages/Secrets.svelte";
  import Policies from "./pages/Policies.svelte";
  import Payments from "./pages/Payments.svelte";
  import Evidence from "./pages/Evidence.svelte";
  import Monitor from "./pages/Monitor.svelte";
  import Terminal from "./components/Terminal.svelte";
  import { currentView, hasCompletedOnboarding, terminalOpen } from "./stores/app";
  import { onMount, onDestroy } from "svelte";

  let View = Onboarding;
  let bottomTerminalRef: Terminal | undefined;

  function handleTerminalCommand(e: Event) {
    const detail = (e as CustomEvent).detail;
    if (bottomTerminalRef && detail) {
      bottomTerminalRef.write(detail);
    }
  }

  onMount(() => {
    window.addEventListener("vault0-terminal-command", handleTerminalCommand);
  });

  onDestroy(() => {
    window.removeEventListener("vault0-terminal-command", handleTerminalCommand);
  });

  $: if (!$hasCompletedOnboarding) {
    View = $currentView === "setup" ? GuidedSetup : Onboarding;
  } else {
    switch ($currentView) {
      case "dashboard": View = Dashboard; break;
      case "monitor": View = Monitor; break;
      case "secrets": View = Secrets; break;
      case "policies": View = Policies; break;
      case "payments": View = Payments; break;
      case "evidence": View = Evidence; break;
      default: View = Dashboard;
    }
  }

  function nav(view: string) {
    currentView.set(view as any);
  }

  function toggleTerminal() {
    terminalOpen.update(v => !v);
  }

  const navItems = [
    { id: "dashboard", label: "Overview", icon: "📊" },
    { id: "monitor", label: "Monitor", icon: "📡" },
    { id: "secrets", label: "Secrets", icon: "🔑" },
    { id: "payments", label: "Wallet", icon: "💰" },
    { id: "policies", label: "Policies", icon: "🛡" },
    { id: "evidence", label: "Activity", icon: "📋" },
  ];
</script>

{#if !$hasCompletedOnboarding}
  <main class="min-h-screen bg-zinc-950 text-zinc-100">
    <svelte:component this={View} />
  </main>
{:else}
  <div class="flex h-screen bg-zinc-950 text-zinc-100">
    <!-- Side Nav -->
    <nav class="w-[200px] shrink-0 flex flex-col border-r border-zinc-800 bg-zinc-950">
      <div class="flex items-center gap-2 px-4 py-4 border-b border-zinc-800">
        <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-emerald-600/20 text-sm font-bold text-emerald-400">V0</div>
        <span class="text-sm font-semibold">Vault-0</span>
      </div>

      <div class="flex-1 py-2 space-y-0.5">
        {#each navItems as item}
          <button
            type="button"
            class="w-full flex items-center gap-3 px-4 py-2.5 text-sm transition-colors {$currentView === item.id ? 'bg-zinc-800/80 text-white' : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40'}"
            on:click={() => nav(item.id)}
          >
            <span class="text-base w-5 text-center">{item.icon}</span>
            <span>{item.label}</span>
          </button>
        {/each}
      </div>

      <div class="border-t border-zinc-800 py-2">
        <button
          type="button"
          class="w-full flex items-center gap-3 px-4 py-2.5 text-sm transition-colors {$terminalOpen ? 'bg-zinc-800/80 text-white' : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40'}"
          on:click={toggleTerminal}
        >
          <span class="text-base w-5 text-center">⌨</span>
          <span>Terminal</span>
        </button>
      </div>
    </nav>

    <!-- Main Area -->
    <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
      <!-- Page Content -->
      <div class="flex-1 min-h-0 overflow-y-auto">
        <svelte:component this={View} />
      </div>

      <!-- Bottom Terminal Panel -->
      {#if $terminalOpen}
        <div class="border-t border-zinc-700 bg-zinc-900" style="height: 35vh; min-height: 150px;">
          <div class="flex items-center justify-between border-b border-zinc-800 px-3 py-1.5">
            <span class="text-xs text-zinc-400 font-medium">Terminal</span>
            <button
              type="button"
              class="text-xs text-zinc-500 hover:text-zinc-300 px-1"
              on:click={toggleTerminal}
            >
              ✕
            </button>
          </div>
          <div class="h-[calc(100%-28px)]">
            <Terminal bind:this={bottomTerminalRef} logPrefix="BottomTerminal" />
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}
