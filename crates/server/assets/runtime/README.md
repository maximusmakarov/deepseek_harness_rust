# Shared host runtime (tier B/C-lite/C1/C2/C2.1-C2.5, rounds 78-97)

The Web UI serves one versioned, immutable module-table runtime to client-half
iframes so a half does not have to bundle its own copy
(`dsh.client.host_modules`). React/ReactDOM are the upstream production UMD
builds, unmodified; `bootstrap.js` is our glue that publishes the reference
module ids (including a jsx-runtime shim over `React.createElement`) in
`window.__DSHR_MODULES__`. `primitives.js` is the design-system shim, ported
in round 93 from the reference repository (`deepseek-ai/deepseek-harness @
39163dd`, `packages/client/ui-primitives/src`): the atoms follow the reference
DOM/props/ARIA/behaviour (`Button`, `Input`, `Switch`, `Checkbox`, `Tag`,
`Pill`, `SegmentedControl`, `SegmentedTabs`, `DisclosureRow`, `TextShimmer`,
`StateDot`, `PathLabel`, `Toast`, `Modal`, `Menu`, `MenuSurface`, `Tooltip`),
and all 204 reference `Icon*` exports are generated from the upstream
`icons/artwork.ts` artwork (inlined instead of the document sprite). Styling
uses the reference `--dsw-alias-*` tokens with fallbacks. The Tooltip placement engine, Menu core (rows/selection/keyboard), submenu
side-card/single-page layout, 200 ms hover grace, MenuGroup/observeStickyMenuGroups
and ShortcutKeys are ported. JsonBlock is a reference port; MarkdownText exposes the reference prop surface
with a dependency-free GFM-subset renderer (the upstream micromark/mdast,
shiki, KaTeX, footnote and incremental-streaming stack is not bundled).
Remaining partial: the detailed useMenuSubmenu focus-transition/page-anchor
mechanics (dshr uses its own CSS/JS placement).

| File | Version | Source | SHA-256 |
|---|---|---|---|
| `react-18.3.1.production.min.js` | 18.3.1 | https://unpkg.com/react@18.3.1/umd/react.production.min.js | `d949f1c3687aedadcedac85261865f29b17cd273997e7f6b2bfc53b2f9d4c4dd` |
| `react-dom-18.3.1.production.min.js` | 18.3.1 | https://unpkg.com/react-dom@18.3.1/umd/react-dom.production.min.js | `35f4f974f4b2bcd44da73963347f8952e341f83909e4498227d4e26b98f66f0d` |
| `bootstrap.js` (ours) | — | this repository | `965290c47106b1f0d6025b566a1860c0b5e436aceef5dcba4ee5867f0ca74c7c` |
| `primitives.js` (ours, reference port) | — | this repository (ported from `deepseek-ai/deepseek-harness @ 39163dd`, `ui-primitives/src`) | `6fbd6afcbd290bea62b29b3f78f1e7ba3e46474f969d077d3918af9e0885f1fe` |

License: MIT (React, Copyright (c) Facebook, Inc. and its affiliates); the
license header is kept in both upstream files. The files are shipped as opaque
browser assets, not linked into Rust code.

Served concatenated as `GET /v1/runtime/react-18.3.1.js` with
`Cache-Control: public, max-age=31536000, immutable`, so every frame of a page
references the same artifact and the browser downloads it once.

Verify the vendored bytes: `scripts/check-runtime-assets.sh`.
