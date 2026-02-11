<script lang="ts">
  export let events: Array<{ ts: string; kind: string; msg: string }> = [];
  export let maxVisible = 50;

  let expandedIdx: number | null = null;

  function toggleExpand(i: number) {
    expandedIdx = expandedIdx === i ? null : i;
  }

  function kindColor(kind: string): string {
    switch (kind) {
      case "allowed": return "border-emerald-500 bg-emerald-500";
      case "blocked": return "border-red-500 bg-red-500";
      case "payment": return "border-amber-500 bg-amber-500";
      default: return "border-blue-500 bg-blue-500";
    }
  }

  function kindBadge(kind: string): string {
    switch (kind) {
      case "allowed": return "text-emerald-400 bg-emerald-950/50";
      case "blocked": return "text-red-400 bg-red-950/50";
      case "payment": return "text-amber-400 bg-amber-950/50";
      default: return "text-blue-400 bg-blue-950/50";
    }
  }

  function formatTs(ts: string): string {
    const secs = parseFloat(ts);
    if (isNaN(secs)) return ts;
    const d = new Date(secs * 1000);
    return d.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  $: visibleEvents = events.slice(-maxVisible).reverse();
</script>

<div class="space-y-0">
  {#each visibleEvents as event, i}
    <div class="relative flex gap-3 {i === 0 && event.kind === 'blocked' ? 'animate-pulse' : ''}">
      <div class="flex flex-col items-center">
        <div class="h-3 w-3 rounded-full border-2 {kindColor(event.kind)}"></div>
        {#if i < visibleEvents.length - 1}
          <div class="w-0.5 flex-1 bg-zinc-800"></div>
        {/if}
      </div>
      <button
        type="button"
        class="mb-2 flex-1 rounded-lg border border-zinc-800 bg-zinc-900/30 px-3 py-2 text-left transition hover:bg-zinc-800/50"
        on:click={() => toggleExpand(i)}
      >
        <div class="flex items-center gap-2">
          <span class="rounded px-1.5 py-0.5 text-xs font-medium {kindBadge(event.kind)}">{event.kind}</span>
          <span class="text-xs text-zinc-500">{formatTs(event.ts)}</span>
        </div>
        <p class="mt-1 text-sm text-zinc-300 {expandedIdx === i ? '' : 'truncate'}">{event.msg}</p>
      </button>
    </div>
  {:else}
    <div class="rounded-lg border border-zinc-800 bg-zinc-900/30 p-6 text-center">
      <p class="text-sm text-zinc-500">No events yet. Start the proxy and run an agent.</p>
    </div>
  {/each}
</div>
