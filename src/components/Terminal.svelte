<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal as XTerm } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";
  import { invoke } from "@tauri-apps/api/core";

  export let rows = 30;
  export let cols = 120;
  export let onOutput: ((text: string) => void) | null = null;
  export let onExit: ((exitCode: number) => void) | null = null;
  export let logPrefix = "Terminal";
  export let inputEnabled = true;

  let container: HTMLDivElement;
  let term: XTerm | null = null;
  let fitAddon: FitAddon | null = null;
  let ptyId: number | null = null;
  let running = false;
  let _resizeObserver: ResizeObserver | null = null;
  const decoder = new TextDecoder();

  async function fitAndResize() {
    if (!term || !fitAddon || ptyId === null) return;
    fitAddon.fit();
    try {
      await invoke("plugin:pty|resize", {
        pid: ptyId,
        cols: term.cols,
        rows: term.rows,
      });
    } catch (e) {
      console.error(`[${logPrefix}] resize error`, e);
    }
  }

  onMount(async () => {
    term = new XTerm({
      cursorBlink: true,
      fontSize: 18,
      fontFamily: "ui-monospace, monospace",
      theme: { background: "#18181b", foreground: "#e4e4e7" },
    });
    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(container);

    const isWin = typeof navigator !== "undefined" && navigator.userAgent.toLowerCase().includes("windows");
    const shell = isWin ? "powershell.exe" : "/bin/zsh";
    const args = isWin ? [] : ["-l"];

    try {
      const spawnArgs = {
        file: shell,
        args,
        termName: "Terminal",
        cols: cols || term.cols,
        rows: rows || term.rows,
        cwd: null,
        env: {},
        encoding: null,
        handleFlowControl: null,
        flowControlPause: null,
        flowControlResume: null,
      };
      console.log(`[${logPrefix}] spawning PTY`, spawnArgs);
      ptyId = await invoke<number>("plugin:pty|spawn", spawnArgs);
      console.log(`[${logPrefix}] PTY spawned`, { ptyId });
      running = true;

      await fitAndResize();
      void readLoop();
      void waitForExit();

      term.onData((data) => {
        if (!inputEnabled) return;
        if (ptyId !== null) {
          invoke("plugin:pty|write", { pid: ptyId, data }).catch((e) => {
            console.error(`[${logPrefix}] write error`, e);
          });
        }
      });

      const resizeObserver = new ResizeObserver(() => {
        void fitAndResize();
      });
      resizeObserver.observe(container);
      _resizeObserver = resizeObserver;
    } catch (e) {
      console.error(`[${logPrefix}] spawn failed`, e);
      if (term) term.writeln("Failed to start shell: " + String(e));
    }
  });

  async function readLoop() {
    while (running && ptyId !== null && term) {
      try {
        const data = await invoke<number[]>("plugin:pty|read", { pid: ptyId });
        if (data && term) {
          const text = decoder.decode(new Uint8Array(data));
          term.write(text);
          if (onOutput) onOutput(text);
        }
      } catch (e) {
        const msg = String(e);
        if (!msg.includes("EOF")) {
          console.error(`[${logPrefix}] PTY read error`, e);
          if (term) term.writeln(`\r\nPTY read error: ${msg}`);
        }
        break;
      }
    }
  }

  async function waitForExit() {
    if (ptyId === null) return;
    try {
      const code = await invoke<number>("plugin:pty|exitstatus", { pid: ptyId });
      console.log(`[${logPrefix}] PTY exit`, { code });
      if (onExit) onExit(code);
    } catch (e) {
      console.error(`[${logPrefix}] exitstatus error`, e);
    }
  }

  onDestroy(() => {
    running = false;
    _resizeObserver?.disconnect();
    if (ptyId !== null) {
      invoke("plugin:pty|kill", { pid: ptyId }).catch((e) => {
        console.error(`[${logPrefix}] kill error`, e);
      });
    }
    if (term) term.dispose();
    term = null;
    fitAddon = null;
    ptyId = null;
  });

  export function write(data: string) {
    if (ptyId !== null) {
      invoke("plugin:pty|write", { pid: ptyId, data });
    }
  }
</script>

<div class="terminal-container h-full w-full overflow-hidden rounded-lg bg-zinc-900" bind:this={container}></div>
