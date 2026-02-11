import { writable } from "svelte/store";

export type ViewName =
  | "welcome"
  | "setup"
  | "dashboard"
  | "secrets"
  | "policies"
  | "payments"
  | "evidence";

export const currentView = writable<ViewName>("welcome");

export const hasCompletedOnboarding = writable(false);

export const terminalOpen = writable(false);

export interface SetupState {
  step: "setup";
  existingPath: string;
  useExisting: boolean;
}

export const setupState = writable<SetupState>({
  step: "setup",
  existingPath: "",
  useExisting: false,
});
