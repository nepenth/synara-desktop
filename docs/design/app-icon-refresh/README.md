# Synara app icon refresh

Status: shipped in Synara 2.1.16.

## Why the current icon underperforms

The desktop master is an irregular transparent silhouette with a large amount of
unused canvas and several thin, luminous details. Dock and application-grid
renderers scale the entire canvas, so the recognizable part of the mark appears
substantially smaller than adjacent apps. At notification and menu sizes, the
thin orbit, connectors, highlights, and feather edges collapse into visual noise.

## Platform constraints

### Apple platforms

Apple's current app-icon guidance favors a simple, memorable core idea with
minimal shapes, centered content, and enough safe space for system masking. iOS,
iPadOS, and macOS use a square 1024×1024 layout that the system masks into its
rounded shape. The system should provide edge treatment and dynamic visual
effects; baked-in masks, bevels, glows, and shadows can conflict with those
effects. Current platforms can also display default, dark, clear, and tinted
appearances while preserving the same recognizable visual identity.

References:

- https://developer.apple.com/design/human-interface-guidelines/app-icons
- https://developer.apple.com/design/resources/
- https://developer.apple.com/documentation/xcode/configuring-your-app-icon

### Linux desktops

Linux doesn't provide one universal adaptive-icon mask. GNOME recommends a
full-color scalable SVG or a 256×256 PNG installed under the `hicolor` icon
theme using the application identifier, plus an optional symbolic SVG for high
contrast. A 48×48 application icon is the baseline and 256×256 is strongly
recommended for the application grid. Synara should therefore ship a deliberate
Linux color asset with transparent outer corners, fixed-size raster fallbacks,
and a simple monochrome symbolic variant rather than relying on the Apple square
master unchanged.

References:

- https://developer.gnome.org/documentation/guidelines/maintainer/integrating.html#icons
- https://developer.gnome.org/documentation/tutorials/themed-icons.html#application-icons

## Superseded replacement concepts

- **A — Orbital Wing:** Closest continuity with the current identity. Strong
  improvement in visual mass; the central ring and orbit still add some detail.
- **B — Signal S:** Strongest pure silhouette at 16 px, but loses too much of the
  existing wing/network identity and reads as an abstract letter.
- **C — Constellation Wing:** Best overall balance. It preserves the network and
  wing, has the cleanest negative space, and remains recognizable at small sizes.
- **D — Connected Conversation:** Communicates messaging most directly, but is
  more generic and its speech/wing combination is less compositionally stable.

After review, these replacement directions were rejected in favor of preserving
the existing Synara identity. They remain here only as design-history evidence.

## Faithful refinements

- **E — Faithful Balanced:** Preserves the anatomy while increasing visual mass
  and simplifying fragile highlights.
- **F — Bold Cleanup:** Uses the strongest structural lines and clearest
  small-size silhouette; useful as the legibility reference.
- **G — Restrained Depth:** Preserves more of the original dimensional character
  with controlled lighting.
- **H — Conservative Recomposition:** Closest to the intended direction: keep
  the current geometry and character, enlarge it in the tile, strengthen only
  disappearing details, and remove small specular noise.

The production direction is **the current icon, faithfully recomposed**. The
shipping large-size master uses H's conservative composition, while compact
assets use F's stronger structural weight so the mark stays legible in a Dock,
taskbar, launcher, notification, or Settings list. The rejected alternatives
remain design-history evidence rather than alternate product identities.

## Production implementation

- `assets/branding/synara-app-icon-master.png` is the unmasked Apple/iOS source.
- `assets/branding/synara-app-icon-small.png` supplies the reinforced small-size
  artwork.
- `assets/branding/synara-app-icon-desktop.png` is the transparent-corner Linux
  source. macOS ICNS representations use opaque, unmasked Apple artwork so the
  system can apply its own shape; Windows ICO uses the reinforced compact master.
- `assets/branding/synara-symbolic.svg` supplies a one-color Linux high-contrast
  icon installed into the standard hicolor symbolic directory.
- `scripts/generate-app-icons.swift` deterministically exports the iOS catalog,
  desktop PNG family, ICNS, and ICO from those reviewed sources.
- `scripts/check-app-icons.mjs` validates dimensions, color types, alpha rules,
  container contents, symbolic safety, and a reviewed SHA-256 manifest.

## Simplified mark (November 2026)

The detailed mark (thin spokes, four ringed nodes, glow) was replaced by one
flat drawing used at every size: a heavy orbit arc, two solid nodes joined by
spokes to a ring core, and three feathers. The feather roots are cut back in an
even, transparent gap around the ring (a real clip, so it holds on any
background). The sections above describe the earlier artwork and remain as
design history; the pipeline is unchanged.

- `assets/branding/synara-mark.svg` is the colour source, and
  `assets/branding/synara-symbolic.svg` is the same geometry in one colour.
  The in-app component `synara/src/app/pages/auth/SynaraMark.tsx` draws the
  same paths.
- Both PNG masters are that drawing on the existing dark tile (`#030510`),
  rasterised from the SVG at 1254 px. The mark spans 80% of the tile width in
  `synara-app-icon-master.png` and 84% in `synara-app-icon-small.png`.
- Not covered by the generator, and redrawn from the same SVG by hand:
  `synara/public/res/png/synara*.png` and the matching iOS `SynaraLogo*`
  image sets (transparent mark; the unread and highlight variants add a
  white-ringed dot at the top right), `synara/public/favicon.ico`,
  `src-tauri/icons/tray-template.png`, and the WiX installer bitmaps in
  `src-tauri/wix/`.

## Release strategy

An icon-only release still runs packaging, signing/notarization, updater
metadata, and TestFlight upload, proving that the assets reach each distributed
binary. The fail-closed `scripts/ci-icon-only.mjs` classifier substitutes focused
asset and packaging-contract checks for unrelated runtime and Matrix integration
suites. Any non-icon production change automatically restores the full CI path.

## Files

- `concept-a-orbital-wing.png`
- `concept-b-signal-s.png`
- `concept-c-constellation-wing.png`
- `concept-d-connected-conversation.png`
- `platform-preview.png`
- `refinement-e-faithful-balanced.png`
- `refinement-f-bold-cleanup.png`
- `refinement-g-restrained-depth.png`
- `refinement-h-conservative-recomposition.png`
- `refinement-platform-preview.png`

The concept and refinement images were produced with the built-in
image-generation tool using the existing Synara icon as a brand reference. They
are exploratory raster art, not final production masters.
