// Registry for files chosen in the browser. The Tauri dialog API returns local paths; the web dialog shim returns
// opaque `web-pick:<n>` handles instead and keeps the File objects here until the command that uses them runs.
import { WEB_PICK_PREFIX } from "../adapt";

export interface Pick {
  file: File;
  /** Path inside the target folder: the plain file name, or `webkitRelativePath` for a folder pick. */
  rel: string;
}

const picks = new Map<string, Pick>();
let next = 0;

export function registerPick(file: File, rel: string): string {
  const h = `${WEB_PICK_PREFIX}${++next}`;
  picks.set(h, { file, rel });
  return h;
}

export function resolvePicks(handles: string[]): Pick[] {
  return handles.map((h) => {
    const p = picks.get(h);
    if (!p) throw new Error("The selected file is no longer available. Choose it again.");
    return p;
  });
}

export function releasePicks(handles: string[]): void {
  for (const h of handles) picks.delete(h);
}
