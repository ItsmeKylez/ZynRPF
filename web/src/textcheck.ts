/** Helpers for the text editor: find and replace, and an XML well-formedness check. */
export function countMatches(text: string, find: string, matchCase: boolean): number {
  if (!find) return 0;
  const h = matchCase ? text : text.toLowerCase(), n = matchCase ? find : find.toLowerCase();
  let c = 0, i = 0;
  while ((i = h.indexOf(n, i)) !== -1) { c++; i += n.length; }
  return c;
}

/** Replaces every literal occurrence (no patterns). */
export function replaceAll(text: string, find: string, repl: string, matchCase: boolean): { text: string; count: number } {
  if (!find) return { text, count: 0 };
  const h = matchCase ? text : text.toLowerCase(), n = matchCase ? find : find.toLowerCase();
  let out = "", i = 0, count = 0, j: number;
  while ((j = h.indexOf(n, i)) !== -1) { out += text.slice(i, j) + repl; i = j + n.length; count++; }
  return { text: out + text.slice(i), count };
}

export const looksLikeXml = (text: string) => /^\s*<(\?xml|[A-Za-z_])/.test(text);

/** Returns null when the text is well-formed XML, otherwise the browser's error message. */
export function checkXml(text: string): string | null {
  const doc = new DOMParser().parseFromString(text, "application/xml");
  const bad = doc.getElementsByTagName("parsererror")[0];
  if (!bad) return null;
  const msg = (bad.textContent || "").replace(/\s+/g, " ").replace(/^This page contains the following errors:\s*/i, "").replace(/\s*Below is a rendering of the page up to the first error\.?$/i, "").trim();
  return msg.length > 300 ? msg.slice(0, 300) + "..." : msg || "The XML is not well-formed.";
}
