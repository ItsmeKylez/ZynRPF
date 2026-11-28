// Runs async tasks one at a time per key, in the order they were submitted.
//
// Why: the reused desktop views fire reads (e.g. "load this entry") from effects that re-run on every UI re-render, so a
// read can be issued just before the user's edit but, over HTTP, arrive after it and find the old path gone. Per-document
// FIFO ordering makes the server see requests in the order the UI issued them, like the in-process desktop IPC did.
export class KeyedQueue {
  private tails = new Map<string, Promise<unknown>>();

  run<T>(key: string, task: () => Promise<T>): Promise<T> {
    const prev = this.tails.get(key) ?? Promise.resolve();
    const result = prev.then(task, task); // a failed predecessor never blocks or poisons later tasks
    const tail = result.catch(() => undefined);
    this.tails.set(key, tail);
    void tail.then(() => {
      if (this.tails.get(key) === tail) this.tails.delete(key);
    });
    return result;
  }
}
