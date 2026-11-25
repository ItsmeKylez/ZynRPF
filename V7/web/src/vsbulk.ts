// Helpers for the visualsettings.dat editor. Presets, tint and range logic follow the original Zyn-RPF (GPL-3.0); see LICENSE-GPL-NOTICE.md.
export interface Preset { id: string; name: string; help: string; prefixes: string[] }

export const PRESETS: Preset[] = [
  { id: "damage", name: "Damage overlay", help: "The mark at the screen edge when you are hit: color, opacity, length and duration.", prefixes: ["misc.DamageOverlay."] },
  { id: "kill", name: "Kill effect", help: "How long the effect lasts after a kill.", prefixes: ["misc.PedKillOverlay."] },
  { id: "coronas", name: "Light halos and flares", help: "Size, strength and color shift of lamp, headlight and sun flares.", prefixes: ["misc.coronas.", "lensflare."] },
  { id: "vehicle", name: "Vehicle lights", help: "Color, range and intensity of headlights, brake lights, indicators and neons.", prefixes: ["car.headlight.", "car.taillight.", "car.indicator.", "car.reversinglight.", "car.defaultlight.", "car.brakelight.", "car.neon.", "car.coronas.", "car.sirens.", "car.siren"] },
  { id: "distant", name: "Distant city lights", help: "Size and color of far-away road, street and building lights.", prefixes: ["distantlights.", "lodlight.", "trafficLight.", "trafficlight."] },
  { id: "bloom", name: "Bloom, exposure and tone", help: "Glow spill, eye adaptation and tone mapping.", prefixes: ["bloom.", "Exposure.", "Adaptation.", "Tonemapping.", "misc.BloomIntensityClamp.", "misc.CrossPMultiplier."] },
  { id: "sky", name: "Sky and clouds", help: "Sky, moon and cloud layers.", prefixes: ["sky.", "cloud.", "cloudgen.", "skyplane.", "misc.MoonDimMult"] },
  { id: "rain", name: "Rain and puddles", help: "Raindrops and ground puddles.", prefixes: ["rain.", "puddles.", "raincollision."] },
  { id: "ped", name: "Character light", help: "Extra light on the player and NPCs, including inside vehicles.", prefixes: ["ped.", "pedlight.", "pedLight.", "pedFpslight.", "pedFpsLight.", "pedincar.", "rim."] },
  { id: "blur", name: "Blur and depth of field", help: "Blur while aiming, when hit, and in cutscenes.", prefixes: ["adaptivedof.", "misc.HiDof.", "misc.DOFBlurMultiplier.", "defaultmotionblur.", "cam.", "misc.SniperRifleDof."] },
];

export const matchesPreset = (p: Preset, key: string) => { const k = key.toLowerCase(); return p.prefixes.some((x) => k.startsWith(x.toLowerCase())); };

/** "750.0f" -> 750 */
export const parseNum = (v: string) => parseFloat(v.replace(/[fF]$/, ""));
export const formatNum = (v: number, scale255: boolean) => (scale255 ? String(Math.round(v)) : String(Number(v.toFixed(4))));

export const toHex = (rgb: [number, number, number]) => "#" + rgb.map((c) => Math.round(Math.min(255, Math.max(0, c))).toString(16).padStart(2, "0")).join("");
export const fromHex = (h: string): [number, number, number] => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];

export function rgbHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255; g /= 255; b /= 255;
  const mx = Math.max(r, g, b), mn = Math.min(r, g, b), l = (mx + mn) / 2;
  if (mx === mn) return [0, 0, l];
  const d = mx - mn, s = l > 0.5 ? d / (2 - mx - mn) : d / (mx + mn);
  const h = mx === r ? (g - b) / d + (g < b ? 6 : 0) : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h * 60, s, l];
}
export function hslRgb(h: number, s: number, l: number): [number, number, number] {
  if (s === 0) return [l * 255, l * 255, l * 255];
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q;
  const k = (t: number) => { t = ((t % 1) + 1) % 1; return t < 1 / 6 ? p + (q - p) * 6 * t : t < 1 / 2 ? q : t < 2 / 3 ? p + (q - p) * (2 / 3 - t) * 6 : p; };
  h /= 360;
  return [k(h + 1 / 3) * 255, k(h) * 255, k(h - 1 / 3) * 255];
}
/** Takes the hue and saturation of the target and keeps the brightness of the current color. */
export function moveHue(current: [number, number, number], target: [number, number, number]): [number, number, number] {
  const [, , l] = rgbHsl(...current);
  const [h, s, tl] = rgbHsl(...target);
  return hslRgb(h, s, l === 0 ? tl : l);
}

/** Slider range for a numeric setting. */
export function numRange(key: string, v: number): [number, number] {
  const k = key.toLowerCase(), a = Math.abs(v);
  if (/duration|time|ms\b/.test(k)) return [0, Math.max(2000, a * 3)];
  if (/enabled|^.*\.use|\.do[A-Z_]/i.test(key) && (v === 0 || v === 1)) return [0, 1];
  const top = a === 0 ? 1 : a <= 1 ? Math.max(1, a * 3) : a * 3;
  return [v < 0 ? -top : 0, top];
}

/** Request body: one "key<TAB>value" line per change. */
export const changeLines = (pending: Map<string, string>) => [...pending].map(([k, v]) => `${k}\t${v}`).join("\n");

/** Tints every colour triple in the list toward the target color, keeping each one's brightness. Returns the number of triples changed. */
export function tintColours(cols: { keys: [string, string, string]; scale: 1 | 255 }[], valueOf: (k: string) => string, target: [number, number, number], strength: number, out: Map<string, string>): number {
  let n = 0;
  for (const c of cols) {
    const cur = c.keys.map((k) => (parseNum(valueOf(k)) * 255) / c.scale) as [number, number, number];
    if (cur.some((x) => !Number.isFinite(x))) continue;
    const moved = moveHue(cur, target);
    const next = cur.map((x, i) => x + (moved[i] - x) * strength);
    c.keys.forEach((k, i) => out.set(k, formatNum((next[i] * c.scale) / 255, c.scale === 255)));
    n++;
  }
  return n;
}
