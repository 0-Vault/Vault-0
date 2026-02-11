import { writable } from "svelte/store";

export interface LogEvent {
  ts: string;
  kind: "allowed" | "blocked" | "payment" | "info";
  msg: string;
}

export const events = writable<LogEvent[]>([]);
