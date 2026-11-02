// Mirrors iOS `SynaraAvatarPalette.hex(for:)` (synara-ios/Synara/SharedUI/SynaraDesign.swift)
// so a person keeps the same fallback colour on every device: 64-bit FNV-1a
// over the UTF-8 bytes of the lower-cased seed, then the high 32 bits modulo
// the eight palette slots. `--mx-uc-1…8` hold the iOS hues in that order
// (212, 248, 285, 330, 8, 28, 140, 195).

const PALETTE_SIZE = 8;
const UINT32 = 0x1_0000_0000;
const encoder = new TextEncoder();

/** Index into the shared eight-hue palette, identical to the iOS bucket. */
export function mxidPaletteIndex(seed: string): number {
  // The 64-bit hash as two unsigned 32-bit halves; offset basis 0xcbf29ce484222325.
  let hi = 0xcbf29ce4;
  let lo = 0x84222325;
  encoder.encode(seed.toLowerCase()).forEach((byte) => {
    // eslint-disable-next-line no-bitwise
    lo = (lo ^ byte) >>> 0;
    // Multiply by the FNV prime 0x100000001b3 = 2^40 + 0x1b3, mod 2^64. Every
    // partial product stays below 2^53, so plain doubles are exact.
    const low = lo * 0x1b3;
    const carry = Math.floor(low / UINT32);
    hi = (hi * 0x1b3 + carry + lo * 0x100) % UINT32;
    lo = low % UINT32;
  });
  return hi % PALETTE_SIZE;
}

export function cssColorMXID(userId: string): string {
  return `--mx-uc-${mxidPaletteIndex(userId) + 1}`;
}

export default function colorMXID(userId: string): string {
  return `var(${cssColorMXID(userId)})`;
}
