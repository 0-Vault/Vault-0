<script lang="ts">
  import { currentView } from "../stores/app";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface Policy {
    allow_domains: string[];
    block_domains: string[];
    spend_cap_cents: number | null;
    output_redact_patterns: string[];
    auto_settle_402: boolean;
  }

  let policy: Policy = {
    allow_domains: [],
    block_domains: [],
    spend_cap_cents: null,
    output_redact_patterns: [],
    auto_settle_402: false,
  };

  let newAllow = "";
  let newBlock = "";
  let newRedact = "";
  let saving = false;
  let status = "";

  onMount(async () => {
    try {
      policy = await invoke("load_policy", { path: null });
    } catch (_) {}
  });

  function addAllow() {
    const v = newAllow.trim();
    if (!v || policy.allow_domains.includes(v)) return;
    policy.allow_domains = [...policy.allow_domains, v];
    newAllow = "";
  }

  function removeAllow(d: string) {
    policy.allow_domains = policy.allow_domains.filter((x) => x !== d);
  }

  function addBlock() {
    const v = newBlock.trim();
    if (!v || policy.block_domains.includes(v)) return;
    policy.block_domains = [...policy.block_domains, v];
    newBlock = "";
  }

  function removeBlock(d: string) {
    policy.block_domains = policy.block_domains.filter((x) => x !== d);
  }

  function addRedact() {
    const v = newRedact.trim();
    if (!v || policy.output_redact_patterns.includes(v)) return;
    policy.output_redact_patterns = [...policy.output_redact_patterns, v];
    newRedact = "";
  }

  function removeRedact(p: string) {
    policy.output_redact_patterns = policy.output_redact_patterns.filter((x) => x !== p);
  }

  async function save() {
    saving = true;
    status = "";
    try {
      await invoke("save_policy", { path: null, policy });
      status = "Saved and applied.";
    } catch (e: any) {
      status = "Error: " + String(e);
    }
    saving = false;
  }

  function applyPreset(name: string) {
    if (name === "openclaw-safe") {
      policy = {
        allow_domains: ["api.openai.com", "api.anthropic.com"],
        block_domains: ["169.254.169.254"],
        spend_cap_cents: 1000,
        output_redact_patterns: ["sk-[a-zA-Z0-9]{20,}", "Bearer [a-zA-Z0-9._-]+"],
        auto_settle_402: false,
      };
    } else if (name === "strict-prod") {
      policy = {
        allow_domains: [],
        block_domains: ["169.254.169.254", "internal.company.local"],
        spend_cap_cents: 500,
        output_redact_patterns: ["sk-[a-zA-Z0-9]{20,}", "Bearer [a-zA-Z0-9._-]+", "ghp_[a-zA-Z0-9]+"],
        auto_settle_402: false,
      };
    } else {
      policy = {
        allow_domains: [],
        block_domains: [],
        spend_cap_cents: null,
        output_redact_patterns: [],
        auto_settle_402: false,
      };
    }
  }
</script>

<div class="flex min-h-screen flex-col">
  <header class="flex items-center gap-4 border-b border-zinc-800 px-6 py-4">
    <button type="button" class="text-sm text-zinc-400 hover:text-white" on:click={() => currentView.set("dashboard")}>
      Back
    </button>
    <h1 class="text-lg font-semibold">Policies</h1>
    <div class="ml-auto flex gap-2">
      <button type="button" class="rounded border border-zinc-600 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-800" on:click={() => applyPreset("openclaw-safe")}>OpenClaw Safe</button>
      <button type="button" class="rounded border border-zinc-600 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-800" on:click={() => applyPreset("strict-prod")}>Strict Prod</button>
      <button type="button" class="rounded border border-zinc-600 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-800" on:click={() => applyPreset("permissive")}>Permissive</button>
    </div>
  </header>

  <section class="flex-1 space-y-6 overflow-y-auto p-6">
    <div>
      <h2 class="mb-2 text-sm font-medium text-zinc-300">Allowed domains</h2>
      <p class="mb-2 text-xs text-zinc-500">Empty = all domains allowed. Add entries to restrict.</p>
      <div class="flex gap-2">
        <input type="text" bind:value={newAllow} placeholder="e.g. api.openai.com" class="flex-1 rounded border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-white placeholder-zinc-500 focus:border-emerald-500 focus:outline-none" on:keydown={(e) => e.key === "Enter" && addAllow()} />
        <button type="button" class="rounded bg-zinc-700 px-3 py-1.5 text-sm text-white hover:bg-zinc-600" on:click={addAllow}>Add</button>
      </div>
      <div class="mt-2 flex flex-wrap gap-1">
        {#each policy.allow_domains as d}
          <span class="inline-flex items-center gap-1 rounded bg-emerald-900/50 px-2 py-0.5 text-xs text-emerald-300">
            {d}
            <button type="button" class="text-emerald-400 hover:text-white" on:click={() => removeAllow(d)}>x</button>
          </span>
        {/each}
      </div>
    </div>

    <div>
      <h2 class="mb-2 text-sm font-medium text-zinc-300">Blocked domains</h2>
      <div class="flex gap-2">
        <input type="text" bind:value={newBlock} placeholder="e.g. internal.company.local" class="flex-1 rounded border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-white placeholder-zinc-500 focus:border-emerald-500 focus:outline-none" on:keydown={(e) => e.key === "Enter" && addBlock()} />
        <button type="button" class="rounded bg-zinc-700 px-3 py-1.5 text-sm text-white hover:bg-zinc-600" on:click={addBlock}>Add</button>
      </div>
      <div class="mt-2 flex flex-wrap gap-1">
        {#each policy.block_domains as d}
          <span class="inline-flex items-center gap-1 rounded bg-red-900/50 px-2 py-0.5 text-xs text-red-300">
            {d}
            <button type="button" class="text-red-400 hover:text-white" on:click={() => removeBlock(d)}>x</button>
          </span>
        {/each}
      </div>
    </div>

    <div>
      <h2 class="mb-2 text-sm font-medium text-zinc-300">Spend cap (cents)</h2>
      <input
        type="number"
        bind:value={policy.spend_cap_cents}
        placeholder="e.g. 1000 = $10.00"
        class="w-48 rounded border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-white placeholder-zinc-500 focus:border-emerald-500 focus:outline-none"
      />
    </div>

    <div>
      <h2 class="mb-2 text-sm font-medium text-zinc-300">Output redaction patterns (regex)</h2>
      <div class="flex gap-2">
        <input type="text" bind:value={newRedact} placeholder="e.g. sk-[a-zA-Z0-9]&#123;20,&#125;" class="flex-1 rounded border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-white placeholder-zinc-500 focus:border-emerald-500 focus:outline-none" on:keydown={(e) => e.key === "Enter" && addRedact()} />
        <button type="button" class="rounded bg-zinc-700 px-3 py-1.5 text-sm text-white hover:bg-zinc-600" on:click={addRedact}>Add</button>
      </div>
      <div class="mt-2 flex flex-wrap gap-1">
        {#each policy.output_redact_patterns as p}
          <span class="inline-flex items-center gap-1 rounded bg-amber-900/50 px-2 py-0.5 font-mono text-xs text-amber-300">
            {p}
            <button type="button" class="text-amber-400 hover:text-white" on:click={() => removeRedact(p)}>x</button>
          </span>
        {/each}
      </div>
    </div>

    <div>
      <label class="flex items-center gap-2 text-sm text-zinc-300">
        <input type="checkbox" bind:checked={policy.auto_settle_402} />
        Auto-settle x402 payments (without confirmation)
      </label>
    </div>

    <div class="flex items-center gap-4">
      <button
        type="button"
        class="rounded bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
        disabled={saving}
        on:click={save}
      >
        {saving ? "Saving…" : "Save & Apply"}
      </button>
      {#if status}
        <span class="text-xs {status.startsWith('Error') ? 'text-red-400' : 'text-emerald-400'}">{status}</span>
      {/if}
    </div>
  </section>
</div>
