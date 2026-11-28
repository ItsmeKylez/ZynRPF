// Vite alias target for "@tauri-apps/api/event". The web backend has no progress events yet (jobs/SSE come later),
// so a listener simply never fires; returning a no-op unlisten keeps the existing effect cleanup working.
export async function listen<T>(_event: string, _handler: (e: { payload: T }) => void): Promise<() => void> {
  return () => {};
}
