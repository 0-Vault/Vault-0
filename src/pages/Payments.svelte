<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface WalletInfo { has_wallet: boolean; address: string; balance_cents: number; network: string; }
  interface Pending402 { url: string; amount_cents: number; currency: string; }

  let wallet: WalletInfo | null = null;
  let pending: Pending402[] = [];
  let error = "";
  let creating = false;
  let importMode = false;
  let mnemonic = "";
  let importError = "";
  let showSeed = false;
  let exportedSeed = "";
  let exporting = false;

  async function load() {
    try {
      wallet = await invoke<WalletInfo>("get_wallet_info");
      pending = await invoke<Pending402[]>("get_pending_402");
    } catch (_) {}
  }

  onMount(load);

  async function createWallet() {
    creating = true; error = "";
    try {
      const result = await invoke<{ info: WalletInfo; recovery_phrase: string }>("create_wallet");
      wallet = result.info;
      exportedSeed = result.recovery_phrase;
      showSeed = true;
    } catch (e) { error = String(e); }
    creating = false;
  }

  async function importWallet() {
    importError = "";
    if (!mnemonic.trim()) { importError = "Enter a mnemonic phrase."; return; }
    try {
      await invoke("import_wallet", { mnemonicPhrase: mnemonic.trim() });
      mnemonic = ""; importMode = false;
      await load();
    } catch (e) { importError = String(e); }
  }

  async function exportSeed() {
    exporting = true;
    try {
      const result = await invoke<{ phrase: string }>("export_seed");
      exportedSeed = result.phrase;
      showSeed = true;
    } catch (e) { error = String(e); }
    exporting = false;
  }
</script>

<div class="p-6 space-y-5 max-w-4xl mx-auto">
  <h1 class="text-xl font-bold">Wallet</h1>

  {#if !wallet?.has_wallet}
    <!-- No wallet yet -->
    <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-6 space-y-4 max-w-md mx-auto text-center">
      <div class="text-4xl">💰</div>
      <h2 class="text-base font-semibold">No Wallet Created</h2>
      <p class="text-sm text-zinc-400">Give your agent its own wallet for autonomous x402 micropayments (USDC on Base). This is optional.</p>
      <div class="space-y-2">
        <button
          class="w-full rounded-xl bg-emerald-600 px-5 py-3 text-base font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
          disabled={creating}
          on:click={createWallet}
        >
          {creating ? "Creating..." : "Create Wallet"}
        </button>
        <button
          class="w-full rounded-xl border border-zinc-600 px-5 py-3 text-sm text-zinc-300 hover:bg-zinc-800"
          on:click={() => (importMode = !importMode)}
        >
          {importMode ? "Cancel Import" : "Import Existing Wallet"}
        </button>
      </div>
      {#if importMode}
        <div class="space-y-2 text-left">
          <label for="import-mnemonic" class="text-xs text-zinc-500">Mnemonic recovery phrase</label>
          <textarea
            id="import-mnemonic"
            bind:value={mnemonic}
            rows="3"
            placeholder="word1 word2 word3 ..."
            class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white font-mono focus:border-emerald-500 focus:outline-none"
          ></textarea>
          {#if importError}
            <p class="text-sm text-red-400">{importError}</p>
          {/if}
          <button class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-500" on:click={importWallet}>
            Import
          </button>
        </div>
      {/if}
      {#if error}
        <p class="text-sm text-red-400">{error}</p>
      {/if}
    </div>
  {:else}
    <!-- Wallet exists -->
    <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-5 space-y-3">
      <div class="flex items-center gap-2">
        <span class="text-xl">💰</span>
        <h2 class="text-base font-semibold">Your Agent Wallet</h2>
      </div>
      <div class="grid grid-cols-2 gap-3 text-sm">
        <div>
          <span class="text-xs text-zinc-500">Address</span>
          <p class="font-mono text-zinc-200 break-all text-xs">{wallet.address}</p>
        </div>
        <div>
          <span class="text-xs text-zinc-500">Balance</span>
          <p class="text-xl font-bold text-white">${(wallet.balance_cents / 100).toFixed(2)} <span class="text-xs text-zinc-500">USDC</span></p>
        </div>
        <div>
          <span class="text-xs text-zinc-500">Network</span>
          <p class="text-zinc-200">{wallet.network}</p>
        </div>
      </div>
    </div>

    {#if pending.length > 0}
      <div class="rounded-xl border border-amber-800 bg-amber-950/20 p-5 space-y-3">
        <h2 class="text-base font-semibold text-amber-300">Pending x402 Payments</h2>
        <div class="space-y-2">
          {#each pending as p}
            <div class="flex items-center justify-between rounded-lg bg-zinc-800/50 px-3 py-2 text-sm">
              <span class="font-mono text-zinc-300 text-xs break-all">{p.url}</span>
              <span class="text-amber-400">${(p.amount_cents / 100).toFixed(2)} {p.currency}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-5 space-y-3">
      <h2 class="text-base font-semibold">Wallet Actions</h2>
      <div class="grid grid-cols-2 gap-2">
        <button
          class="rounded-lg border border-zinc-700 px-4 py-3 text-sm text-zinc-300 hover:bg-zinc-800 text-left disabled:opacity-50"
          disabled={exporting}
          on:click={exportSeed}
        >
          🔐 Export Seed Phrase
        </button>
        <button
          class="rounded-lg border border-zinc-700 px-4 py-3 text-sm text-zinc-300 hover:bg-zinc-800 text-left"
          on:click={() => (importMode = !importMode)}
        >
          📥 Import Wallet
        </button>
      </div>
    </div>

    {#if showSeed && exportedSeed}
      <div class="rounded-xl border border-amber-800 bg-amber-950/20 p-5 space-y-3">
        <h3 class="text-sm font-semibold text-amber-300">Recovery Phrase</h3>
        <p class="text-xs text-amber-400">Write this down and store it safely. Anyone with this phrase can access your funds.</p>
        <div class="rounded-lg bg-zinc-800 p-3 font-mono text-sm text-amber-100 break-words">{exportedSeed}</div>
        <button class="text-xs text-zinc-400 hover:text-zinc-300" on:click={() => { showSeed = false; exportedSeed = ""; }}>
          Hide seed phrase
        </button>
      </div>
    {/if}

    {#if importMode}
      <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-5 space-y-3">
        <h3 class="text-sm font-semibold">Import Wallet</h3>
        <label for="import-mnemonic-2" class="text-xs text-zinc-500">Mnemonic recovery phrase</label>
        <textarea
          id="import-mnemonic-2"
          bind:value={mnemonic}
          rows="3"
          placeholder="word1 word2 word3 ..."
          class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white font-mono focus:border-emerald-500 focus:outline-none"
        ></textarea>
        {#if importError}
          <p class="text-sm text-red-400">{importError}</p>
        {/if}
        <button class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-500" on:click={importWallet}>
          Import
        </button>
      </div>
    {/if}

    {#if error}
      <p class="text-sm text-red-400">{error}</p>
    {/if}
  {/if}
</div>
