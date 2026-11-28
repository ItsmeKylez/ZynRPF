// Vite alias target for "@tauri-apps/plugin-opener".
import { notifyUnavailable } from "../unavailable";

export async function openUrl(url: string): Promise<void> {
  // Only http(s) links; anything else (file:, javascript:, ...) is refused.
  if (!/^https?:\/\//i.test(url)) throw new Error("Only http and https links can be opened.");
  window.open(url, "_blank", "noopener,noreferrer");
}

export async function openPath(_path: string): Promise<void> {
  notifyUnavailable("Opening a folder");
}

export async function revealItemInDir(_path: string): Promise<void> {
  notifyUnavailable("Showing a file in its folder");
}
