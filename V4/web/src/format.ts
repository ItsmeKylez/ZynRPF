export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const u = ["KiB", "MiB", "GiB", "TiB"];
  let v = n / 1024, i = 0;
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v < 10 ? v.toFixed(2) : v < 100 ? v.toFixed(1) : v.toFixed(0)} ${u[i]}`;
}
export const hex = (n: number, w: number) => n.toString(16).toUpperCase().padStart(w, "0");
export function hexDump(bytes: Uint8Array, base: number): string {
  const lines: string[] = [];
  for (let o = 0; o < bytes.length; o += 16) {
    const row = bytes.subarray(o, o + 16);
    let h = "", a = "";
    for (let i = 0; i < 16; i++) {
      if (i < row.length) { h += hex(row[i], 2) + " "; a += row[i] >= 0x20 && row[i] < 0x7f ? String.fromCharCode(row[i]) : "."; }
      else h += "   ";
      if (i === 7) h += " ";
    }
    lines.push(`${hex(base + o, 8)}  ${h} ${a}`);
  }
  return lines.join("\n");
}
export function parseOffset(s: string): number | null {
  const t = s.trim();
  if (/^0x[0-9a-f]+$/i.test(t)) return parseInt(t.slice(2), 16);
  if (/^[0-9]+$/.test(t)) return parseInt(t, 10);
  return null;
}
