/** RSC7 resource header (first 16 bytes of a stored resource entry): magic "RSC7", version, system flags, graphics flags.
 *  The size decoding follows the community-documented flag layout. It is NOT verified against real game files. */
export const RSC7_MAGIC = 0x37435352; // bytes 52 53 43 37 = "RSC7"

export interface Rsc7 {
  magic: boolean;
  version: number;
  sysFlags: number;
  gfxFlags: number;
  sysSize: number;
  gfxSize: number;
  /** Flags in the archive's entry table differ from the header's flags. */
  flagsMatchEntry: boolean | null;
}

/** Memory size that a resource flag word describes. */
export function sizeFromFlags(flags: number): number {
  const f = flags >>> 0;
  const parts =
    (((f >>> 27) & 1) << 0) + (((f >>> 26) & 1) << 1) + (((f >>> 25) & 1) << 2) + (((f >>> 24) & 1) << 3) +
    (((f >>> 17) & 0x7f) << 4) + (((f >>> 11) & 0x3f) << 5) + (((f >>> 7) & 0xf) << 6) + (((f >>> 5) & 3) << 7) + (((f >>> 4) & 1) << 8);
  return 0x200 * 2 ** (f & 0xf) * parts;
}

/** Reads the header from at least 16 bytes; returns null when fewer bytes are given. */
export function parseRsc7(b: Uint8Array, entrySys: number | null, entryGfx: number | null): Rsc7 | null {
  if (b.length < 16) return null;
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const sysFlags = v.getUint32(8, true), gfxFlags = v.getUint32(12, true);
  return {
    magic: v.getUint32(0, true) === RSC7_MAGIC,
    version: v.getUint32(4, true),
    sysFlags, gfxFlags,
    sysSize: sizeFromFlags(sysFlags), gfxSize: sizeFromFlags(gfxFlags),
    flagsMatchEntry: entrySys === null || entryGfx === null ? null : (entrySys >>> 0) === sysFlags && (entryGfx >>> 0) === gfxFlags,
  };
}
