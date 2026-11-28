// Central place for "this desktop feature has no web backend yet". Never fakes success: callers get an
// explicit error (commands) or a visible notice (dialogs/opener), and the shell shows it as a toast.
export const UNAVAILABLE_EVENT = "zyn:unavailable";

export class UnavailableError extends Error {
  readonly code = "NOT_AVAILABLE_IN_WEB";
  constructor(what: string) {
    super(`${what} is not available in the web version yet.`);
    this.name = "UnavailableError";
  }
}

export function notifyUnavailable(what: string): void {
  const message = new UnavailableError(what).message;
  if (typeof window !== "undefined") window.dispatchEvent(new CustomEvent(UNAVAILABLE_EVENT, { detail: message }));
}
