// Particle editor helpers (pure functions). Ported from the original Zyn-RPF `yptToplu.ts` (GPL-3.0, Copyright (c) 2026 Talkdedsec); see LICENSE-GPL-NOTICE.md.
import type { YptGroup } from "./api";

export type Value = [number, number, number, number];

export interface BulkSettings {
  /** hue shift in degrees */
  hue?: number;
  /** tint colour as #rrggbb, strength 0..1 */
  tint?: string;
  strength?: number;
  /** multipliers */
  size?: number;
  light?: number;
  density?: number;
}

export const isColour = (name: string) => /rgb|colourTint/i.test(name);
export const isSize = (name: string) => /whdM|sizeScalar|zoomScalar|coronaSize|dimensions/i.test(name);
export const isLight = (name: string) => /Light:m_intensity|coronaIntensity|emissiveIntensity/i.test(name);
export const isDensity = (name: string) => /spawnRate/i.test(name);

/** Shifts the hue while keeping brightness; also works for colours above 1 (HDR). */
export function shiftHue([r, g, b]: number[], degrees: number): [number, number, number] {
  const peak = Math.max(r, g, b, 1e-6);
  const [x, y, z] = [r / peak, g / peak, b / peak];
  const hi = Math.max(x, y, z);
  const lo = Math.min(x, y, z);
  const l = (hi + lo) / 2;
  const d = hi - lo;
  if (d < 1e-6) return [r, g, b];
  const s = d / (1 - Math.abs(2 * l - 1));
  let h = hi === x ? ((y - z) / d) % 6 : hi === y ? (z - x) / d + 2 : (x - y) / d + 4;
  h = (((h * 60 + degrees) % 360) + 360) % 360;
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const xx = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  const [a, bb, cc] = h < 60 ? [c, xx, 0] : h < 120 ? [xx, c, 0] : h < 180 ? [0, c, xx] : h < 240 ? [0, xx, c] : h < 300 ? [xx, 0, c] : [c, 0, xx];
  return [(a + m) * peak, (bb + m) * peak, (cc + m) * peak];
}

const hexToRgb = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);

/** Applies the settings to every matching keyframe of the groups. Only changed values are written to `pending`; returns how many changed. */
export function applyBulk(groups: YptGroup[], s: BulkSettings, pending: Map<number, Value>): number {
  const [hr, hg, hb] = s.tint ? hexToRgb(s.tint) : [0, 0, 0];
  const strength = s.tint ? (s.strength ?? 1) : 0;
  const mul = (d: Value, k = 1): Value => (k === 1 ? d : [d[0] * k, d[1] * k, d[2] * k, d[3] * k]);
  let n = 0;
  for (const g of groups)
    for (const p of g.properties)
      for (const f of p.frames) {
        let d = [...(pending.get(f.offset) ?? f.value)] as Value;
        const before = d.join();
        if (isColour(p.name)) {
          if (s.hue) d = [...shiftHue(d, s.hue), d[3]];
          if (strength > 0) {
            const peak = Math.max(d[0], d[1], d[2], 1e-6);
            d = [d[0] + (hr * peak - d[0]) * strength, d[1] + (hg * peak - d[1]) * strength, d[2] + (hb * peak - d[2]) * strength, d[3]];
          }
        }
        if (isSize(p.name)) d = mul(d, s.size);
        if (isLight(p.name)) d = mul(d, s.light);
        if (isDensity(p.name)) d = mul(d, s.density);
        if (d.join() !== before) {
          pending.set(f.offset, d);
          n++;
        }
      }
  return n;
}

/** Every word must appear in the name or the behaviour. */
export function filterGroups(groups: YptGroup[], query: string, kind?: YptGroup["kind"] | "all"): YptGroup[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return groups.filter((g) => (!kind || kind === "all" || g.kind === kind) && words.every((w) => g.name.toLowerCase().includes(w) || (g.behaviour ?? "").toLowerCase().includes(w)));
}

/** Request body for the server: one "offset v0 v1 v2 v3" line per change. */
export function changeLines(pending: Map<number, Value>): string {
  return [...pending].map(([o, v]) => `${o} ${v.join(" ")}`).join("\n") + "\n";
}
