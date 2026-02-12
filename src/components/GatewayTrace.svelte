<script lang="ts">
  export let events: Array<{
    ts: string;
    kind: string;
    session_id: string;
    platform: string;
    summary: string;
    payload: string;
  }> = [];
  export let maxVisible = 80;

  let expandedIdx: number | null = null;

  function toggleExpand(i: number) {
    expandedIdx = expandedIdx === i ? null : i;
  }

  function kindColor(kind: string): string {
    switch (kind) {
      case "message_in": return "border-blue-500 bg-blue-500";
      case "message_out": return "border-emerald-500 bg-emerald-500";
      case "tool_call": return "border-purple-500 bg-purple-500";
      case "tool_result": return "border-violet-500 bg-violet-500";
      case "thinking": return "border-amber-500 bg-amber-500";
      case "error": return "border-red-500 bg-red-500";
      default: return "border-zinc-500 bg-zinc-500";
    }
  }

  function kindBadge(kind: string): string {
    switch (kind) {
      case "message_in": return "text-blue-400 bg-blue-950/50";
      case "message_out": return "text-emerald-400 bg-emerald-950/50";
      case "tool_call": return "text-purple-400 bg-purple-950/50";
      case "tool_result": return "text-violet-400 bg-violet-950/50";
      case "thinking": return "text-amber-400 bg-amber-950/50";
      case "error": return "text-red-400 bg-red-950/50";
      default: return "text-zinc-400 bg-zinc-950/50";
    }
  }

  function kindLabel(kind: string): string {
    switch (kind) {
      case "message_in": return "IN";
      case "message_out": return "OUT";
      case "tool_call": return "TOOL";
      case "tool_result": return "RESULT";
      case "thinking": return "THINK";
      case "error": return "ERROR";
      default: return kind.toUpperCase();
    }
  }

  function platformBadge(platform: string): string {
    switch (platform.toLowerCase()) {
      case "whatsapp": return "bg-green-900/40 text-green-400";
      case "telegram": return "bg-sky-900/40 text-sky-400";
      case "discord": return "bg-indigo-900/40 text-indigo-400";
      case "slack": return "bg-orange-900/40 text-orange-400";
      default: return "bg-zinc-800 text-zinc-400";
    }
  }

  function formatTs(ts: string): string {
    const secs = parseFloat(ts);
    if (isNaN(secs)) return ts;
    const d = new Date(secs * 1000);
    return d.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  function formatPayload(raw: string): string {
    try {
      return JSON.stringify(JSON.parse(raw), null, 2);
    } catch {
      return raw;
    }
  }

  $: visibleEvents = events.slice(-maxVisible).reverse();
</script>

<div class="space-y-0">
  {#each visibleEvents as event, i}
    <div class="relative flex gap-3">
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
        <div class="flex items-center gap-2 flex-wrap">
          <span class="rounded px-1.5 py-0.5 text-[10px] font-bold {kindBadge(event.kind)}">{kindLabel(event.kind)}</span>
          {#if event.platform}
            <span class="rounded px-1.5 py-0.5 text-[10px] {platformBadge(event.platform)}">{event.platform}</span>
          {/if}
          <span class="text-[10px] text-zinc-600">{formatTs(event.ts)}</span>
          {#if event.session_id}
            <span class="text-[10px] text-zinc-700 font-mono">{event.session_id.slice(0, 8)}</span>
          {/if}
        </div>
        <p class="mt-1 text-sm text-zinc-300 {expandedIdx === i ? '' : 'truncate'}">{event.summary}</p>
        {#if expandedIdx === i}
          <pre class="mt-2 rounded bg-zinc-950 border border-zinc-800 p-2 text-xs text-zinc-400 overflow-x-auto max-h-48 overflow-y-auto">{formatPayload(event.payload)}</pre>
        {/if}
      </button>
    </div>
  {:else}
    <div class="rounded-lg border border-zinc-800 bg-zinc-900/30 p-6 text-center">
      <p class="text-sm text-zinc-500">No agent activity yet. Connect to the gateway to start monitoring.</p>
    </div>
  {/each}
</div>
