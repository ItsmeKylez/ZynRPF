/** Text decoding helpers for the text editor. Strict: anything that is not clean UTF-8 text is refused, so binary data is never rewritten. */
export interface Decoded { text: string; bom: boolean; eol: "\n" | "\r\n" }
export const MAX_TEXT_BYTES = 1024 * 1024;

export function decodeText(b: Uint8Array): Decoded | null {
  if (b.includes(0)) return null;
  let bom = false, body = b;
  if (b.length >= 3 && b[0] === 0xef && b[1] === 0xbb && b[2] === 0xbf) { bom = true; body = b.subarray(3); }
  let t: string;
  try { t = new TextDecoder("utf-8", { fatal: true }).decode(body); } catch { return null; }
  const crlf = (t.match(/\r\n/g) || []).length, lf = (t.match(/\n/g) || []).length - crlf;
  const eol = crlf > lf ? "\r\n" : "\n";
  // Normalise to \n for editing; encodeText restores the original line ending. Lone \r would be lost, so refuse files that have them.
  if (/\r(?!\n)/.test(t)) return null;
  return { text: t.replace(/\r\n/g, "\n"), bom, eol };
}

export function encodeText(text: string, d: { bom: boolean; eol: "\n" | "\r\n" }): Uint8Array {
  const t = d.eol === "\r\n" ? text.replace(/\n/g, "\r\n") : text;
  const body = new TextEncoder().encode(t);
  if (!d.bom) return body;
  const out = new Uint8Array(body.length + 3);
  out.set([0xef, 0xbb, 0xbf]); out.set(body, 3);
  return out;
}
