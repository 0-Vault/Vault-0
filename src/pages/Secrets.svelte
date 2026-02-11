<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface VaultEntryInfo { alias: string; provider: string; preview: string; created_at: string; }
  interface NewSecretFound { key_name: string; file: string; provider: string; preview: string; }

  let entries: VaultEntryInfo[] = [];
  let newSecrets: NewSecretFound[] = [];
  let migrating = "";
  let vaultUnlocked = false;
  let unlockPassphrase = "";
  let unlockError = "";
  let unlocking = false;

  let revealedValues: Record<string, string> = {};
  let showAdd = false;
  let newAlias = "";
  let newProvider = "openai";
  let newValue = "";
  let addError = "";
  let adding = false;

  let deleteConfirm = "";
  let deleting = false;

  const PROVIDERS = ["openai", "anthropic", "grok", "telegram", "slack", "discord", "github", "custom"];

  async function load() {
    try {
      vaultUnlocked = await invoke<boolean>("vault_is_unlocked");
      if (vaultUnlocked) {
        entries = await invoke<VaultEntryInfo[]>("vault_list_entries");
        newSecrets = await invoke<NewSecretFound[]>("scan_for_new_secrets");
      }
    } catch (_) {}
  }

  async function migrateSecret(ns: NewSecretFound) {
    migrating = ns.key_name;
    try {
      const home = (await invoke<string>("detect_openclaw")).toString();
      // Re-run harden for this specific key
      await invoke("secure_config_keys", {
        installPath: home,
        keysToSecure: [[ns.key_name.toLowerCase().replace(/ /g, "_"), ns.key_name]],
      });
      await load();
    } catch (_) {}
    migrating = "";
  }

  onMount(load);

  async function unlock() {
    unlockError = "";
    unlocking = true;
    try {
      await invoke("vault_unlock", { passphrase: unlockPassphrase });
      vaultUnlocked = true;
      await load();
    } catch (e) { unlockError = String(e); }
    unlocking = false;
  }

  async function toggleReveal(alias: string) {
    if (revealedValues[alias]) {
      delete revealedValues[alias];
      revealedValues = { ...revealedValues };
    } else {
      try {
        const val = await invoke<string>("vault_get_secret", { alias });
        revealedValues = { ...revealedValues, [alias]: val };
      } catch (_) {}
    }
  }

  async function addSecret() {
    addError = "";
    if (!newAlias.trim() || !newValue.trim()) { addError = "Alias and value are required."; return; }
    adding = true;
    try {
      await invoke("vault_add_entry", { alias: newAlias.trim(), value: newValue.trim(), provider: newProvider });
      newAlias = ""; newValue = ""; newProvider = "openai"; showAdd = false;
      await load();
    } catch (e) { addError = String(e); }
    adding = false;
  }

  async function deleteSecret(alias: string) {
    deleting = true;
    try {
      await invoke("vault_delete_entry", { alias });
      deleteConfirm = "";
      delete revealedValues[alias];
      revealedValues = { ...revealedValues };
      await load();
    } catch (_) {}
    deleting = false;
  }
</script>

<div class="p-6 space-y-5 max-w-4xl mx-auto">
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-bold">Secrets</h1>
    {#if vaultUnlocked}
      <button
        class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-500"
        on:click={() => (showAdd = !showAdd)}
      >
        {showAdd ? "Cancel" : "+ Add Secret"}
      </button>
    {/if}
  </div>

  {#if !vaultUnlocked}
    <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-6 space-y-3 max-w-md mx-auto">
      <div class="flex items-center gap-2">
        <span class="text-xl">🔒</span>
        <h2 class="text-base font-semibold">Vault Locked</h2>
      </div>
      <p class="text-sm text-zinc-400">Enter your master passphrase to view and manage secrets.</p>
      <input
        type="password"
        bind:value={unlockPassphrase}
        placeholder="Master passphrase"
        class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-3 text-base text-white focus:border-emerald-500 focus:outline-none"
      />
      {#if unlockError}
        <p class="text-sm text-red-400">{unlockError}</p>
      {/if}
      <button
        class="w-full rounded-xl bg-emerald-600 px-5 py-3 text-base font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
        disabled={unlocking}
        on:click={unlock}
      >
        {unlocking ? "Unlocking..." : "Unlock Vault"}
      </button>
    </div>
  {:else}
    {#if showAdd}
      <div class="rounded-xl border border-emerald-800 bg-zinc-900/60 p-5 space-y-3">
        <h3 class="text-sm font-semibold text-emerald-300">Add New Secret</h3>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="new-alias" class="text-xs text-zinc-500 mb-1 block">Alias (variable name)</label>
            <input
              id="new-alias"
              bind:value={newAlias}
              placeholder="e.g. OPENAI_API_KEY"
              class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white font-mono focus:border-emerald-500 focus:outline-none"
            />
          </div>
          <div>
            <label for="new-provider" class="text-xs text-zinc-500 mb-1 block">Provider</label>
            <select
              id="new-provider"
              bind:value={newProvider}
              class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white focus:border-emerald-500 focus:outline-none"
            >
              {#each PROVIDERS as p}
                <option value={p}>{p}</option>
              {/each}
            </select>
          </div>
        </div>
        <div>
          <label for="new-value" class="text-xs text-zinc-500 mb-1 block">Secret value</label>
          <input
            id="new-value"
            type="password"
            bind:value={newValue}
            placeholder="Enter API key or token"
            class="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-white focus:border-emerald-500 focus:outline-none"
          />
        </div>
        {#if addError}
          <p class="text-sm text-red-400">{addError}</p>
        {/if}
        <button
          class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
          disabled={adding}
          on:click={addSecret}
        >
          {adding ? "Adding..." : "Add to Vault"}
        </button>
      </div>
    {/if}

    {#if newSecrets.length > 0}
      <div class="rounded-xl border border-amber-800 bg-amber-950/20 p-5 space-y-3">
        <div class="flex items-center gap-2">
          <span class="text-lg">⚠</span>
          <h3 class="text-sm font-semibold text-amber-300">New Plaintext Secrets Detected</h3>
        </div>
        <p class="text-xs text-zinc-400">OpenClaw added these keys to disk. Migrate them to your encrypted vault to remove plaintext exposure.</p>
        <div class="space-y-2">
          {#each newSecrets as ns}
            <div class="flex items-center justify-between rounded-lg bg-zinc-800/50 px-3 py-2">
              <div class="flex items-center gap-2">
                <span class="font-mono text-sm text-amber-200">{ns.key_name}</span>
                <span class="rounded bg-zinc-700 px-1.5 py-0.5 text-[10px] text-zinc-400">{ns.provider}</span>
                <span class="text-xs text-zinc-500">in {ns.file}</span>
                <span class="font-mono text-xs text-zinc-600">{ns.preview}</span>
              </div>
              <button
                class="rounded bg-amber-600 px-3 py-1 text-xs font-semibold text-white hover:bg-amber-500 disabled:opacity-50"
                disabled={migrating === ns.key_name}
                on:click={() => migrateSecret(ns)}
              >
                {migrating === ns.key_name ? "Migrating..." : "Migrate"}
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    {#if entries.length === 0}
      <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 p-6 text-center">
        <p class="text-sm text-zinc-400">No secrets stored yet.</p>
        <p class="text-xs text-zinc-500 mt-1">Click "+ Add Secret" to store your first API key or token.</p>
      </div>
    {:else}
      <div class="rounded-xl border border-zinc-700 bg-zinc-900/60 overflow-hidden">
        <div class="grid grid-cols-[1fr_auto_1fr_auto] gap-0 text-xs text-zinc-500 px-4 py-2 border-b border-zinc-800">
          <span>Name</span>
          <span>Provider</span>
          <span>Value</span>
          <span>Actions</span>
        </div>
        {#each entries as entry}
          <div class="grid grid-cols-[1fr_auto_1fr_auto] gap-0 items-center px-4 py-3 border-b border-zinc-800/50 hover:bg-zinc-800/30">
            <span class="font-mono text-sm text-zinc-200">{entry.alias}</span>
            <span class="rounded bg-zinc-700 px-2 py-0.5 text-[10px] text-zinc-400 mx-2">{entry.provider}</span>
            <div class="font-mono text-sm">
              {#if revealedValues[entry.alias]}
                <span class="text-emerald-400 break-all">{revealedValues[entry.alias]}</span>
              {:else}
                <span class="text-zinc-600">{entry.preview}</span>
              {/if}
            </div>
            <div class="flex items-center gap-1 ml-2">
              <button
                class="rounded px-2 py-1 text-xs text-zinc-400 hover:text-white hover:bg-zinc-700"
                on:click={() => toggleReveal(entry.alias)}
                title={revealedValues[entry.alias] ? "Hide" : "Show"}
              >
                {revealedValues[entry.alias] ? "🙈" : "👁"}
              </button>
              {#if deleteConfirm === entry.alias}
                <button
                  class="rounded px-2 py-1 text-xs text-red-400 hover:bg-red-900/30 disabled:opacity-50"
                  disabled={deleting}
                  on:click={() => deleteSecret(entry.alias)}
                >
                  Confirm
                </button>
                <button
                  class="rounded px-2 py-1 text-xs text-zinc-400 hover:bg-zinc-700"
                  on:click={() => (deleteConfirm = "")}
                >
                  Cancel
                </button>
              {:else}
                <button
                  class="rounded px-2 py-1 text-xs text-zinc-400 hover:text-red-400 hover:bg-zinc-700"
                  on:click={() => (deleteConfirm = entry.alias)}
                  title="Delete"
                >
                  🗑
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>
