<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  export let visible = false;
  export let promptLabel = "Sensitive input";
  export let promptKind: "provider_api_key" | "telegram_token" | "slack_token" | "custom" = "custom";

  const dispatch = createEventDispatcher();
  let alias = "openai";
  let value = "";
  let busy = false;
  let error = "";

  $: if (promptKind === "telegram_token") alias = "telegram";
  $: if (promptKind === "slack_token") alias = "slack";
  $: if (promptKind === "provider_api_key" && alias !== "openai" && alias !== "anthropic" && alias !== "grok") alias = "openai";

  async function storeAndContinue() {
    error = "";
    if (!value.trim()) {
      error = "Enter a value first.";
      return;
    }
    if (!alias.trim()) {
      error = "Alias is required.";
      return;
    }
    busy = true;
    try {
      await invoke("set_secret", { alias: alias.trim(), value: value.trim() });
      const aliasToken = `VAULT0_ALIAS:${alias.trim()}`;
      value = "";
      dispatch("stored", { aliasToken });
    } catch (e) {
      error = String(e);
    }
    busy = false;
  }

  function close() {
    if (busy) return;
    dispatch("cancel");
  }
</script>

{#if visible}
  <div class="absolute inset-0 z-30 flex items-center justify-center bg-black/70 backdrop-blur-sm">
    <div class="w-full max-w-lg rounded-2xl border border-emerald-800 bg-zinc-950 p-5 shadow-2xl">
      <div class="mb-3 flex items-center gap-2 text-emerald-300">
        <span class="text-xl">🔒</span>
        <h3 class="text-lg font-semibold">Vault-0 Secure Input</h3>
      </div>
      <p class="mb-3 text-sm text-zinc-300">
        OpenClaw is requesting: <span class="font-semibold text-white">{promptLabel}</span>
      </p>
      <p class="mb-4 text-sm text-zinc-400">
        This value is stored in macOS Keychain and injected as a `VAULT0_ALIAS` token.
      </p>

      <label for="secure-alias" class="mb-2 block text-sm text-zinc-400">Alias</label>
      <input
        id="secure-alias"
        class="mb-3 w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:border-emerald-500 focus:outline-none"
        bind:value={alias}
        placeholder="openai"
      />

      <label for="secure-value" class="mb-2 block text-sm text-zinc-400">Secret value</label>
      <input
        id="secure-value"
        type="password"
        class="w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm text-white focus:border-emerald-500 focus:outline-none"
        bind:value={value}
        placeholder="Enter key/token"
      />

      {#if error}
        <p class="mt-3 text-sm text-red-400">{error}</p>
      {/if}

      <div class="mt-4 flex gap-2">
        <button
          type="button"
          class="rounded-lg border border-zinc-600 px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-800"
          on:click={close}
        >
          Cancel
        </button>
        <button
          type="button"
          class="ml-auto rounded-lg bg-emerald-600 px-4 py-2 text-sm font-semibold text-white hover:bg-emerald-500 disabled:opacity-50"
          on:click={storeAndContinue}
          disabled={busy}
        >
          {busy ? "Storing..." : "Store Securely + Continue"}
        </button>
      </div>
    </div>
  </div>
{/if}
