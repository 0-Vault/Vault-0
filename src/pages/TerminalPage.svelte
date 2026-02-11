<script lang="ts">
  import { currentView } from "../stores/app";
  import Terminal from "../components/Terminal.svelte";

  let terminalRef: Terminal | undefined;

  const PRESETS = [
    {
      label: "Set Up OpenClaw",
      cmd: "if [ -d openclaw ]; then cd openclaw; else git clone https://github.com/openclaw/openclaw.git openclaw && cd openclaw; fi && npx -y pnpm@10.23.0 install && npx -y pnpm@10.23.0 run ui:build\n",
    },
    {
      label: "Set Up IronClaw",
      cmd: "cargo install ironclaw\n",
    },
  ];

  function runPreset(cmd: string) {
    terminalRef?.write(cmd);
  }
</script>

<div class="flex min-h-screen flex-col">
  <header class="flex items-center gap-4 border-b border-zinc-800 px-6 py-4">
    <button type="button" class="text-sm text-zinc-400 hover:text-white" on:click={() => currentView.set("dashboard")}>
      Back
    </button>
    <h1 class="text-lg font-semibold">Terminal</h1>
    <div class="flex gap-2">
      {#each PRESETS as p}
        <button
          type="button"
          class="rounded border border-zinc-600 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-200 hover:bg-zinc-700"
          on:click={() => runPreset(p.cmd)}
        >
          {p.label}
        </button>
      {/each}
    </div>
  </header>
  <section class="flex-1 min-h-0 p-4">
    <Terminal bind:this={terminalRef} />
  </section>
</div>
