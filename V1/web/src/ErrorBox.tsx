import { ApiError } from "./api";
import { m } from "./messages";

export interface Shown { title?: string; message: string; next?: string; code?: string; requestId?: string }
export const toShown = (e: unknown, title?: string): Shown =>
  e instanceof ApiError ? { title, message: e.message, next: e.next, code: e.code, requestId: e.requestId } : { title, message: m.networkError, next: m.networkErrorNext };

export function ErrorBox({ err, onRetry, retryLabel }: { err: Shown; onRetry?: () => void; retryLabel?: string }) {
  return (
    <div className="error" role="alert">
      {err.title && <h3>{err.title}</h3>}
      <p>{err.message}</p>
      {err.next && <p><strong>{m.nextStepLabel}:</strong> {err.next}</p>}
      {(err.code || err.requestId) && (
        <p className="meta">
          {err.code && <span>{m.errorCodeLabel}: <code>{err.code}</code></span>}{" "}
          {err.requestId && <span>{m.requestIdLabel}: <code>{err.requestId}</code></span>}
        </p>
      )}
      {onRetry && <button onClick={onRetry}>{retryLabel || m.retry}</button>}
    </div>
  );
}
