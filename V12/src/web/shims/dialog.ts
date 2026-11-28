// Vite alias target for "@tauri-apps/plugin-dialog". Native dialogs return local paths, which do not exist in a
// browser. `open` shows the browser's file chooser and returns `web-pick:` handles (see picks.ts); "choose an export
// folder" has no web equivalent, so it returns a `web-download:` handle and the export arrives as a ZIP download.
// `save` (save-as to a chosen path) still has no web meaning and says so.
import { EXPORT_DIALOG_TITLE, WEB_DOWNLOAD_HANDLE } from "../adapt";
import { notifyUnavailable } from "../unavailable";
import { registerPick } from "./picks";

interface OpenOptions {
  multiple?: boolean;
  directory?: boolean;
  title?: string;
}

type DirInput = HTMLInputElement & { webkitdirectory: boolean };

/** Must be called synchronously from a user gesture (a click handler), or browsers block the chooser. */
function chooseFiles(opts: OpenOptions): Promise<File[] | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input") as DirInput;
    input.type = "file";
    input.multiple = !!opts.multiple && !opts.directory;
    if (opts.directory) input.webkitdirectory = true;
    input.style.display = "none";
    document.body.appendChild(input);
    const done = (files: File[] | null) => {
      input.remove();
      resolve(files);
    };
    input.addEventListener("change", () => done(input.files && input.files.length ? Array.from(input.files) : null));
    input.addEventListener("cancel", () => done(null));
    input.click();
  });
}

export async function open(options?: OpenOptions): Promise<string | string[] | null> {
  if (options?.directory && options.title === EXPORT_DIALOG_TITLE) return WEB_DOWNLOAD_HANDLE;
  const files = await chooseFiles(options ?? {});
  if (!files) return null;
  const handles = files.map((f) => registerPick(f, options?.directory ? ((f as File & { webkitRelativePath?: string }).webkitRelativePath || f.name) : f.name));
  return options?.multiple || options?.directory ? handles : handles[0];
}

export async function save(_options?: unknown): Promise<string | null> {
  notifyUnavailable("Saving to a chosen location");
  return null;
}
