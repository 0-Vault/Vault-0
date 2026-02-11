<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  export let events: Array<{ ts: string; kind: string; msg: string }> = [];
  export let walletAddress = "";
  export let visible = false;

  let copied = false;

  interface Stats {
    total: number;
    allowed: number;
    blocked: number;
    payment: number;
  }

  let stats: Stats = { total: 0, allowed: 0, blocked: 0, payment: 0 };

  $: if (visible) loadStats();

  async function loadStats() {
    try {
      stats = await invoke("get_evidence_stats");
    } catch (_) {}
  }

  $: proofText = generateProofText(stats, walletAddress);

  function generateProofText(s: Stats, addr: string): string {
    let lines = [
      `Vault-0 Secure ClawBot Session`,
      `---`,
      `${s.total} total events | ${s.allowed} allowed | ${s.blocked} blocked | ${s.payment} payments`,
    ];
    if (addr) {
      lines.push(`Agent wallet: ${addr.slice(0, 6)}...${addr.slice(-4)}`);
    }
    lines.push(`Secured by Vault-0 — zero plaintext keys, policy-gated, auditable.`);
    return lines.join("\n");
  }

  $: tweetText = encodeURIComponent(
    `Just ran a secure ClawBot with its own wallet using Vault-0 — ${stats.blocked} threats blocked, ${stats.allowed} actions allowed, zero leaked keys.\n\nAll local, all auditable.`
  );
  $: tweetUrl = `https://x.com/intent/tweet?text=${tweetText}`;

  function copyProof() {
    navigator.clipboard.writeText(proofText);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

{#if visible}
  <div class="rounded-lg border border-zinc-700 bg-zinc-900 p-6 space-y-4">
    <h3 class="text-base font-medium">Share Proof of Your Secure Bot</h3>
    <div class="rounded border border-zinc-800 bg-zinc-950 p-4 font-mono text-xs text-zinc-300 whitespace-pre-wrap">{proofText}</div>
    <div class="flex gap-2">
      <button
        type="button"
        class="rounded bg-zinc-700 px-4 py-2 text-sm text-white hover:bg-zinc-600"
        on:click={copyProof}
      >
        {copied ? "Copied" : "Copy Proof"}
      </button>
      <a
        href={tweetUrl}
        target="_blank"
        rel="noopener noreferrer"
        class="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-500"
      >
        Post on X
      </a>
    </div>
  </div>
{/if}
