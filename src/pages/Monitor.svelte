<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import GatewayTrace from "../components/GatewayTrace.svelte";
  import SessionTrace from "../components/SessionTrace.svelte";

  interface GatewayEvent {
    ts: string;
    kind: string;
    session_id: string;
    platform: string;
    summary: string;
    payload: string;
  }

  interface GatewayStatus {
    connected: boolean;
    event_count: number;
    gateway_url: string;
  }

  interface LogEntry {
    ts: string;
    kind: string;
    msg: string;
  }

  let gwEvents: GatewayEvent[] = [];
  let secEvents: LogEntry[] = [];
  let status: GatewayStatus = { connected: false, event_count: 0, gateway_url: "" };
  let pollId: ReturnType<typeof setInterval> | null = null;
  let connecting = false;
  let filterPlatform = "";

  // Counters
  $: messageCount = gwEvents.filter(e => e.kind === "message_in" || e.kind === "message_out").length;
  $: toolCount = gwEvents.filter(e => e.kind === "tool_call" || e.kind === "tool_result").length;
  $: thinkCount = gwEvents.filter(e => e.kind === "thinking").length;
  $: errorCount = gwEvents.filter(e => e.kind === "error").length;
  $: blockedCount = secEvents.filter(e => e.kind === "blocked").length;

  // Unique platforms for filter
  $: platforms = [...new Set(gwEvents.map(e => e.platform).filter(Boolean))];

  // Filtered events
  $: filteredGwEvents = filterPlatform
    ? gwEvents.filter(e => e.platform === filterPlatform)
    : gwEvents;

  async function loadAll() {
    try {
      status = await invoke<GatewayStatus>("gateway_status");
      gwEvents = await invoke<GatewayEvent[]>("get_gateway_events");
      secEvents = await invoke<LogEntry[]>("get_evidence_log");
    } catch (_) {}
  }

  async function connect() {
    connecting = true;
    try {
      await invoke("gateway_connect");
    } catch (_) {}
    setTimeout(async () => {
      await loadAll();
      connecting = false;
    }, 1500);
  }

  async function disconnect() {
    try {
      await invoke("gateway_disconnect");
    } catch (_) {}
    await loadAll();
  }

  onMount(() => {
    loadAll();
    pollId = setInterval(loadAll, 2000);
    return () => { if (pollId) clearInterval(pollId); };
  });
</script>

<div class="p-6 space-y-4 max-w-6xl mx-auto h-full flex flex-col">
  <!-- Header -->
  <div class="flex items-center justify-between shrink-0">
    <div class="flex items-center gap-3">
      <h1 class="text-xl font-bold">Mission Control</h1>
      <div class="flex items-center gap-1.5">
        <span class="h-2 w-2 rounded-full {status.connected ? 'bg-emerald-400 animate-pulse' : 'bg-zinc-600'}"></span>
        <span class="text-xs {status.connected ? 'text-emerald-400' : 'text-zinc-500'}">
          {status.connected ? "Live" : "Disconnected"}
        </span>
      </div>
      {#if status.gateway_url}
        <span class="text-[10px] font-mono text-zinc-600">{status.gateway_url}</span>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      {#if platforms.length > 0}
        <select
          class="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs text-zinc-300 focus:outline-none focus:border-zinc-500"
          bind:value={filterPlatform}
        >
          <option value="">All platforms</option>
          {#each platforms as p}
            <option value={p}>{p}</option>
          {/each}
        </select>
      {/if}

      {#if status.connected}
        <button
          class="rounded border border-zinc-600 px-3 py-1 text-xs text-zinc-300 hover:bg-zinc-800"
          on:click={disconnect}
        >
          Disconnect
        </button>
      {:else}
        <button
          class="rounded bg-emerald-600 px-3 py-1 text-xs text-white hover:bg-emerald-500 disabled:opacity-50"
          disabled={connecting}
          on:click={connect}
        >
          {connecting ? "Connecting..." : "Connect"}
        </button>
      {/if}
    </div>
  </div>

  <!-- Stats Bar -->
  <div class="flex items-center gap-4 shrink-0">
    <div class="flex items-center gap-1.5">
      <span class="h-1.5 w-1.5 rounded-full bg-blue-400"></span>
      <span class="text-xs text-zinc-400">{messageCount} messages</span>
    </div>
    <div class="flex items-center gap-1.5">
      <span class="h-1.5 w-1.5 rounded-full bg-purple-400"></span>
      <span class="text-xs text-zinc-400">{toolCount} tools</span>
    </div>
    <div class="flex items-center gap-1.5">
      <span class="h-1.5 w-1.5 rounded-full bg-amber-400"></span>
      <span class="text-xs text-zinc-400">{thinkCount} thoughts</span>
    </div>
    {#if errorCount > 0}
      <div class="flex items-center gap-1.5">
        <span class="h-1.5 w-1.5 rounded-full bg-red-400"></span>
        <span class="text-xs text-red-400">{errorCount} errors</span>
      </div>
    {/if}
    <div class="ml-auto flex items-center gap-1.5">
      <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
      <span class="text-xs text-zinc-400">{secEvents.filter(e => e.kind === "allowed").length} proxied</span>
    </div>
    {#if blockedCount > 0}
      <div class="flex items-center gap-1.5">
        <span class="h-1.5 w-1.5 rounded-full bg-red-400"></span>
        <span class="text-xs text-red-400">{blockedCount} blocked</span>
      </div>
    {/if}
  </div>

  <!-- Two-Column Layout -->
  <div class="flex-1 grid grid-cols-2 gap-4 min-h-0 overflow-hidden">
    <!-- Left: Agent Brain -->
    <div class="flex flex-col min-h-0 rounded-xl border border-zinc-700 bg-zinc-900/40">
      <div class="flex items-center gap-2 px-4 py-2.5 border-b border-zinc-800 shrink-0">
        <span class="text-sm font-semibold text-zinc-200">Agent Brain</span>
        <span class="text-[10px] text-zinc-600">{filteredGwEvents.length} events</span>
      </div>
      <div class="flex-1 overflow-y-auto p-3">
        {#if !status.connected && gwEvents.length === 0}
          <div class="flex flex-col items-center justify-center h-full text-center space-y-3">
            <div class="text-4xl opacity-30">📡</div>
            <p class="text-sm text-zinc-500">Not connected to OpenClaw gateway</p>
            <p class="text-xs text-zinc-600">Click "Connect" to start streaming agent activity in real time.</p>
            <button
              class="rounded bg-emerald-600 px-4 py-2 text-sm text-white hover:bg-emerald-500 disabled:opacity-50"
              disabled={connecting}
              on:click={connect}
            >
              {connecting ? "Connecting..." : "Connect to Gateway"}
            </button>
          </div>
        {:else}
          <GatewayTrace events={filteredGwEvents} />
        {/if}
      </div>
    </div>

    <!-- Right: Security Shield -->
    <div class="flex flex-col min-h-0 rounded-xl border border-zinc-700 bg-zinc-900/40">
      <div class="flex items-center gap-2 px-4 py-2.5 border-b border-zinc-800 shrink-0">
        <span class="text-sm font-semibold text-zinc-200">Security Shield</span>
        <span class="text-[10px] text-zinc-600">{secEvents.length} events</span>
      </div>
      <div class="flex-1 overflow-y-auto p-3">
        {#if secEvents.length === 0}
          <div class="flex flex-col items-center justify-center h-full text-center space-y-3">
            <div class="text-4xl opacity-30">🛡</div>
            <p class="text-sm text-zinc-500">No security events yet</p>
            <p class="text-xs text-zinc-600">Policy decisions, blocked requests, and payments will appear here when your agent makes outbound calls through the proxy.</p>
          </div>
        {:else}
          <SessionTrace events={secEvents} />
        {/if}
      </div>
    </div>
  </div>
</div>
