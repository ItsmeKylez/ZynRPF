import { VersionInfo } from "./api";
import { formatBytes } from "./format";
import { m } from "./messages";

/** Overview of the open archive. `afterBytes` is the size reported by "Check changes" (null until it has been run). */
export function Summary({ version, pending, afterBytes }: { version: VersionInfo | undefined; pending: number; afterBytes: number | null }) {
  if (!version) return null;
  return (
    <section className="panel" aria-labelledby="sum-h">
      <h2 id="sum-h">{m.summaryHeading}</h2>
      <dl className="facts">
        <dt>{m.summaryFile}</dt><dd>{version.filename}</dd>
        <dt>{m.summarySize}</dt><dd>{formatBytes(version.size)}</dd>
        <dt>{m.summaryEntries}</dt><dd>{version.entry_count.toLocaleString("en-US")}</dd>
        <dt>{m.summaryPending}</dt><dd>{pending === 0 ? m.summaryNoPending : String(pending)}</dd>
        {pending > 0 && <><dt>{m.summaryAfterSave}</dt><dd>{afterBytes === null ? m.summaryAfterSaveHint : formatBytes(afterBytes)}</dd></>}
      </dl>
      <p className="muted">{m.summaryNoteUnverified}</p>
    </section>
  );
}
