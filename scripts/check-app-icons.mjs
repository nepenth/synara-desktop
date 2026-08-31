import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { inflateSync } from "node:zlib";
import { pathToFileURL } from "node:url";

const manifestPath = "assets/branding/app-icon-manifest.json";

const desktopPNGs = new Map([
  ["assets/branding/synara-app-icon-desktop.png", 1024],
  ["src-tauri/icons/16x16.png", 16],
  ["src-tauri/icons/24x24.png", 24],
  ["src-tauri/icons/32x32.png", 32],
  ["src-tauri/icons/48x48.png", 48],
  ["src-tauri/icons/64x64.png", 64],
  ["src-tauri/icons/128x128.png", 128],
  ["src-tauri/icons/128x128@2x.png", 256],
  ["src-tauri/icons/256x256.png", 256],
  ["src-tauri/icons/512x512.png", 512],
  ["src-tauri/icons/icon.png", 512],
  ["src-tauri/icons/StoreLogo.png", 50],
  ["src-tauri/icons/Square30x30Logo.png", 30],
  ["src-tauri/icons/Square44x44Logo.png", 44],
  ["src-tauri/icons/Square71x71Logo.png", 71],
  ["src-tauri/icons/Square89x89Logo.png", 89],
  ["src-tauri/icons/Square107x107Logo.png", 107],
  ["src-tauri/icons/Square142x142Logo.png", 142],
  ["src-tauri/icons/Square150x150Logo.png", 150],
  ["src-tauri/icons/Square284x284Logo.png", 284],
  ["src-tauri/icons/Square310x310Logo.png", 310],
]);

const iosSizes = [20, 29, 40, 58, 60, 76, 80, 87, 120, 152, 167, 180, 1024];
const iosPNGs = new Map(
  iosSizes.map((size) => [
    `synara-ios/Synara/Resources/Assets.xcassets/AppIcon.appiconset/AppIcon-${size}.png`,
    size,
  ])
);

const sourcePNGs = new Map([
  ["assets/branding/synara-app-icon-master.png", 1024],
  ["assets/branding/synara-app-icon-small.png", 1024],
]);

const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

const sha256 = (buffer) => createHash("sha256").update(buffer).digest("hex");

const paeth = (left, above, upperLeft) => {
  const estimate = left + above - upperLeft;
  const leftDistance = Math.abs(estimate - left);
  const aboveDistance = Math.abs(estimate - above);
  const upperLeftDistance = Math.abs(estimate - upperLeft);
  if (leftDistance <= aboveDistance && leftDistance <= upperLeftDistance) return left;
  return aboveDistance <= upperLeftDistance ? above : upperLeft;
};

const parsePNG = (path) => {
  const buffer = readFileSync(path);
  if (!buffer.subarray(0, 8).equals(pngSignature)) {
    throw new Error(`${path} is not a PNG`);
  }

  let offset = 8;
  let metadata;
  const idat = [];
  const chunkTypes = new Set();
  while (offset + 12 <= buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("ascii", offset + 4, offset + 8);
    chunkTypes.add(type);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      metadata = {
        width: data.readUInt32BE(0),
        height: data.readUInt32BE(4),
        bitDepth: data[8],
        colorType: data[9],
        interlace: data[12],
      };
    } else if (type === "IDAT") {
      idat.push(data);
    }
    offset += length + 12;
    if (type === "IEND") break;
  }

  if (!metadata) throw new Error(`${path} has no IHDR chunk`);
  if (metadata.bitDepth !== 8 || metadata.interlace !== 0) {
    throw new Error(`${path} must be non-interlaced 8-bit PNG`);
  }
  const channels = metadata.colorType === 6 ? 4 : metadata.colorType === 2 ? 3 : 0;
  if (!channels) throw new Error(`${path} must use RGB or RGBA color`);

  const inflated = inflateSync(Buffer.concat(idat));
  const stride = metadata.width * channels;
  const pixels = Buffer.alloc(stride * metadata.height);
  let inputOffset = 0;
  for (let y = 0; y < metadata.height; y += 1) {
    const filter = inflated[inputOffset++];
    const rowOffset = y * stride;
    for (let x = 0; x < stride; x += 1) {
      const raw = inflated[inputOffset++];
      const left = x >= channels ? pixels[rowOffset + x - channels] : 0;
      const above = y > 0 ? pixels[rowOffset + x - stride] : 0;
      const upperLeft = y > 0 && x >= channels ? pixels[rowOffset + x - stride - channels] : 0;
      let value;
      switch (filter) {
        case 0:
          value = raw;
          break;
        case 1:
          value = raw + left;
          break;
        case 2:
          value = raw + above;
          break;
        case 3:
          value = raw + Math.floor((left + above) / 2);
          break;
        case 4:
          value = raw + paeth(left, above, upperLeft);
          break;
        default:
          throw new Error(`${path} uses unsupported PNG filter ${filter}`);
      }
      pixels[rowOffset + x] = value & 0xff;
    }
  }

  const alphaAt = (x, y) =>
    channels === 4 ? pixels[y * stride + x * channels + 3] : 255;
  return { buffer, ...metadata, channels, alphaAt, chunkTypes };
};

const assertSquarePNG = (path, expectedSize, alphaRule) => {
  const png = parsePNG(path);
  if (png.width !== expectedSize || png.height !== expectedSize) {
    throw new Error(`${path} is ${png.width}×${png.height}; expected ${expectedSize}×${expectedSize}`);
  }
  if (alphaRule === "opaque" && png.colorType !== 2) {
    throw new Error(`${path} must not contain an alpha channel`);
  }
  if (alphaRule === "opaque" && png.chunkTypes.has("tRNS")) {
    throw new Error(`${path} must not contain a tRNS transparency chunk`);
  }
  if (alphaRule === "rounded") {
    if (png.colorType !== 6) throw new Error(`${path} must contain transparency`);
    const last = expectedSize - 1;
    const middle = Math.floor(expectedSize / 2);
    for (const [x, y] of [[0, 0], [last, 0], [0, last], [last, last]]) {
      if (png.alphaAt(x, y) !== 0) throw new Error(`${path} must have transparent outer corners`);
    }
    for (const [x, y] of [[middle, middle], [middle, 0], [0, middle], [last, middle], [middle, last]]) {
      if (png.alphaAt(x, y) !== 255) throw new Error(`${path} must have an opaque center and edge midpoints`);
    }
  }
  return { width: png.width, height: png.height, colorType: png.colorType, sha256: sha256(png.buffer) };
};

const inspectBinaryIcons = () => {
  const icns = readFileSync("src-tauri/icons/icon.icns");
  if (icns.toString("ascii", 0, 4) !== "icns" || icns.readUInt32BE(4) !== icns.length) {
    throw new Error("src-tauri/icons/icon.icns has an invalid ICNS header");
  }
  const icnsTypes = new Set();
  let icnsOffset = 8;
  while (icnsOffset + 8 <= icns.length) {
    const type = icns.toString("ascii", icnsOffset, icnsOffset + 4);
    const length = icns.readUInt32BE(icnsOffset + 4);
    if (length < 8 || icnsOffset + length > icns.length) {
      throw new Error("src-tauri/icons/icon.icns contains an invalid chunk");
    }
    icnsTypes.add(type);
    icnsOffset += length;
  }
  for (const required of ["ic07", "ic08", "ic09", "ic10"]) {
    if (!icnsTypes.has(required)) {
      throw new Error(`src-tauri/icons/icon.icns is missing ${required}`);
    }
  }

  const ico = readFileSync("src-tauri/icons/icon.ico");
  if (ico.readUInt16LE(0) !== 0 || ico.readUInt16LE(2) !== 1 || ico.readUInt16LE(4) < 4) {
    throw new Error("src-tauri/icons/icon.ico has an invalid or incomplete ICO directory");
  }
  const count = ico.readUInt16LE(4);
  const sizes = new Set();
  for (let index = 0; index < count; index += 1) {
    const width = ico[6 + index * 16] || 256;
    const height = ico[7 + index * 16] || 256;
    if (width === height) sizes.add(width);
  }
  for (const required of [16, 24, 32, 48, 64, 256]) {
    if (!sizes.has(required)) throw new Error(`src-tauri/icons/icon.ico is missing ${required}×${required}`);
  }

  return {
    "src-tauri/icons/icon.icns": { sha256: sha256(icns), bytes: icns.length },
    "src-tauri/icons/icon.ico": { sha256: sha256(ico), bytes: ico.length },
  };
};

const inspectSymbolicIcon = () => {
  const path = "assets/branding/synara-symbolic.svg";
  const buffer = readFileSync(path);
  const svg = buffer.toString("utf8");
  if (!/<svg\b[^>]*viewBox="0 0 256 256"/.test(svg)) {
    throw new Error(`${path} must use the reviewed 256×256 viewBox`);
  }
  if (/<(?:script|image|use)\b/i.test(svg) || /(?:href|url)\s*=/i.test(svg)) {
    throw new Error(`${path} must be self-contained and script-free`);
  }
  if (!svg.includes("#2e3436")) {
    throw new Error(`${path} must use the GNOME symbolic foreground color`);
  }
  return { [path]: { sha256: sha256(buffer), bytes: buffer.length } };
};

const assertLinuxPackagingContract = () => {
  const tauri = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
  const bundledIcons = new Set(tauri.bundle?.icon ?? []);
  for (const size of [16, 24, 32, 48, 64, 128, 256, 512]) {
    const expected = size === 128 ? "icons/128x128.png" : `icons/${size}x${size}.png`;
    if (!bundledIcons.has(expected)) {
      throw new Error(`Tauri bundle must include the exact ${size}×${size} Linux icon (${expected})`);
    }
  }
  if (tauri.bundle?.linux?.deb?.files?.["/usr/share/icons/hicolor/scalable/apps/synara-symbolic.svg"] !== "../assets/branding/synara-symbolic.svg") {
    throw new Error("Debian bundle must install Synara's scalable symbolic hicolor icon");
  }

  const pkgbuild = readFileSync("packaging/arch/PKGBUILD", "utf8");
  for (const size of [16, 24, 32, 48, 64, 128, 256, 512]) {
    if (!pkgbuild.includes(`/usr/share/icons/hicolor/${size}x${size}/apps/synara.png`)) {
      throw new Error(`Arch package must install the exact ${size}×${size} hicolor icon`);
    }
  }
  if (!pkgbuild.includes("/usr/share/icons/hicolor/scalable/apps/synara-symbolic.svg")) {
    throw new Error("Arch package must install Synara's scalable symbolic hicolor icon");
  }

  const desktopEntry = readFileSync("packaging/arch/synara.desktop", "utf8");
  if (!/^Icon=synara$/m.test(desktopEntry)) {
    throw new Error("Linux desktop entry must resolve the packaged hicolor icon by name");
  }
  const startupClass = desktopEntry.match(/^StartupWMClass=(.+)$/m)?.[1];
  if (tauri.app?.enableGtkAppId === true) {
    if (startupClass !== tauri.identifier) {
      throw new Error("Linux StartupWMClass must match Tauri's enabled GTK application id");
    }
  } else if (startupClass !== tauri.mainBinaryName) {
    throw new Error("Linux StartupWMClass must match Tauri's main binary ownership route");
  }
};

export const inspectAppIcons = () => {
  assertLinuxPackagingContract();
  const files = {};
  for (const [path, minimumSize] of sourcePNGs) {
    const png = parsePNG(path);
    if (png.width !== png.height || png.width < minimumSize) {
      throw new Error(`${path} must be a square master of at least ${minimumSize}px`);
    }
    files[path] = { width: png.width, height: png.height, colorType: png.colorType, sha256: sha256(png.buffer) };
  }
  for (const [path, size] of desktopPNGs) files[path] = assertSquarePNG(path, size, "rounded");
  for (const [path, size] of iosPNGs) files[path] = assertSquarePNG(path, size, "opaque");
  Object.assign(files, inspectBinaryIcons());
  Object.assign(files, inspectSymbolicIcon());

  const catalog = JSON.parse(
    readFileSync("synara-ios/Synara/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json", "utf8")
  );
  const catalogBuffer = readFileSync(
    "synara-ios/Synara/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json"
  );
  const catalogFiles = new Set((catalog.images || []).map((image) => image.filename).filter(Boolean));
  for (const size of iosSizes) {
    if (!catalogFiles.has(`AppIcon-${size}.png`)) {
      throw new Error(`iOS AppIcon catalog does not reference AppIcon-${size}.png`);
    }
  }
  const requiredCatalogSlots = [
    ["iphone", "20x20", "2x"], ["iphone", "20x20", "3x"],
    ["iphone", "29x29", "2x"], ["iphone", "29x29", "3x"],
    ["iphone", "40x40", "2x"], ["iphone", "40x40", "3x"],
    ["iphone", "60x60", "2x"], ["iphone", "60x60", "3x"],
    ["ipad", "20x20", "1x"], ["ipad", "20x20", "2x"],
    ["ipad", "29x29", "1x"], ["ipad", "29x29", "2x"],
    ["ipad", "40x40", "1x"], ["ipad", "40x40", "2x"],
    ["ipad", "76x76", "1x"], ["ipad", "76x76", "2x"],
    ["ipad", "83.5x83.5", "2x"], ["ios-marketing", "1024x1024", "1x"],
  ];
  for (const [idiom, size, scale] of requiredCatalogSlots) {
    if (!(catalog.images || []).some((image) => image.idiom === idiom && image.size === size && image.scale === scale && image.filename)) {
      throw new Error(`iOS AppIcon catalog is missing ${idiom} ${size} ${scale}`);
    }
  }
  files["synara-ios/Synara/Resources/Assets.xcassets/AppIcon.appiconset/Contents.json"] = {
    sha256: sha256(catalogBuffer),
    bytes: catalogBuffer.length,
  };
  return files;
};

const main = () => {
  const files = inspectAppIcons();
  if (process.argv.includes("--write")) {
    const manifest = {
      schemaVersion: 1,
      generator: "scripts/generate-app-icons.swift",
      files,
    };
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    console.log(`Wrote ${manifestPath} with ${Object.keys(files).length} icon assets.`);
    return;
  }

  const expected = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (expected.schemaVersion !== 1) throw new Error(`${manifestPath} has an unsupported schema`);
  for (const [path, metadata] of Object.entries(files)) {
    const recorded = expected.files?.[path];
    if (!recorded) throw new Error(`${manifestPath} does not cover ${path}`);
    if (recorded.sha256 !== metadata.sha256) throw new Error(`${path} differs from the reviewed icon manifest`);
  }
  const extras = Object.keys(expected.files || {}).filter((path) => !(path in files));
  if (extras.length > 0) throw new Error(`${manifestPath} contains stale assets: ${extras.join(", ")}`);
  console.log(`Verified ${Object.keys(files).length} app icon assets and platform contracts.`);
};

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
