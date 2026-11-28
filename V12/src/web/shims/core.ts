// Vite alias target for "@tauri-apps/api/core" in the web build.
import { webInvoke } from "../transport";

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return (await webInvoke(cmd, args)) as T;
}
