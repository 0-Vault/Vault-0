<script lang="ts">
  import { currentView } from "../stores/app";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface LogEntry {
    ts: string;
    kind: string;
    msg: string;
  }

  interface ReceiptEntry {
    ts: string;
    kind: string;
    msg: string;
    hash: string;
  }

  let entries: LogEntry[] = [];
  let receipt: ReceiptEntry[] | null = null;
  let exported = false;

  async function load() {
    try {
      entries = await invoke("get_evidence_log");
    } catch (_) {
      entries = [];
    }
  }

  onMount(load);

  async function exportReceipt() {
    const triples: [string, string, string][] = entries.map((e) => [e.ts, e.kind, e.msg]);
    try {
      receipt = await invoke("export_receipt", { entries: triples });
      exported = true;
    } catch (_) {
      receipt = null;
    }
  }

  function copyReceipt() {
    if (!receipt) return;
    const text = receipt.map((r) => `${r.ts}\t${r.kind}\t${r.msg}\t${r.hash}`).join("\n");
    navigator.clipboard.writeText(text);
  }
</script>

<div class="flex min-h-screen flex-col">
  <header class="flex items-center gap-4 border-b border-zinc-800 px-6 py-4">
    <button type="button" class="text-sm text-zinc-400 hover:text-white" on:click={() => currentView.set("dashboard")}>
      Back
    </button>
    <h1 class="text-lg font-semibold">Evidence</h1>
  </header>
  <section class="flex-1 space-y-6 p-6">
    <p class="text-zinc-400">Tamper-evident timeline. Export and share receipts.</p>
    <div class="flex gap-2">
      <button
        type="button"
        class="rounded bg-zinc-700 px-3 py-1.5 text-sm text-white hover:bg-zinc-600"
        on:click={load}
      >
        Refresh
      </button>
      <button
        type="button"
        class="rounded bg-emerald-600 px-3 py-1.5 text-sm text-white hover:bg-emerald-500"
        on:click={exportReceipt}
      >
        Export receipt
      </button>
      {#if exported && receipt}
        <button
          type="button"
          class="rounded bg-zinc-600 px-3 py-1.5 text-sm text-white hover:bg-zinc-500"
          on:click={copyReceipt}
        >
          Copy to clipboard
        </button>
      {/if}
    </div>
    <div class="rounded-lg border border-zinc-800 bg-zinc-900/50 p-4 font-mono text-sm">
      {#each entries.slice().reverse() as e}
        <div class="border-b border-zinc-800 py-1.5 last:border-0">
          <span class="text-zinc-500">{e.ts}</span>
          <span class="ml-2 {e.kind === 'blocked' ? 'text-red-400' : e.kind === 'payment' ? 'text-amber-400' : 'text-zinc-300'}">{e.msg}</span>
        </div>
      {:else}
        <div class="text-zinc-500">No log entries.</div>
      {/each}
    </div>
    {#if receipt && receipt.length > 0}
      <div class="rounded-lg border border-zinc-700 p-4">
        <h2 class="text-sm font-medium text-zinc-300">Receipt (hashed)</h2>
        <pre class="mt-2 max-h-48 overflow-auto text-xs text-zinc-400">{receipt.map((r) => `${r.ts} ${r.kind} ${r.hash}`).join("\n")}</pre>
      </div>
    {/if}
  </section>
</div>
