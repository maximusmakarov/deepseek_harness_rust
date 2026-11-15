# Shared host runtime (tier B/C-lite/C1/C2, rounds 78-85)

The Web UI serves one versioned, immutable module-table runtime to client-half
iframes so a half does not have to bundle its own copy
(`dsh.client.host_modules`). React/ReactDOM are the upstream production UMD
builds, unmodified; `bootstrap.js` is our glue that publishes the reference
module ids (including a jsx-runtime shim over `React.createElement`) in
`window.__DSHR_MODULES__`. `primitives.js` is the independent partial
design-system shim (C2: 30+ exports — components, named `Icon*` glyphs with a
Proxy fallback, `useAnchoredPosition`/`useDismissOnOutsidePointer`,
`writeClipboard`); it is written against the props real conformance plugins use,
not a copy of the upstream package.

| File | Version | Source | SHA-256 |
|---|---|---|---|
| `react-18.3.1.production.min.js` | 18.3.1 | https://unpkg.com/react@18.3.1/umd/react.production.min.js | `d949f1c3687aedadcedac85261865f29b17cd273997e7f6b2bfc53b2f9d4c4dd` |
| `react-dom-18.3.1.production.min.js` | 18.3.1 | https://unpkg.com/react-dom@18.3.1/umd/react-dom.production.min.js | `35f4f974f4b2bcd44da73963347f8952e341f83909e4498227d4e26b98f66f0d` |
| `bootstrap.js` (ours) | — | this repository | `965290c47106b1f0d6025b566a1860c0b5e436aceef5dcba4ee5867f0ca74c7c` |
| `primitives.js` (ours, partial shim) | — | this repository (independent implementation, not the upstream package) | `9bfb5d9cc804ec056a0f77eb2ea32c5b7afdc1adfe70c1ded04d772559bbcbff` |

License: MIT (React, Copyright (c) Facebook, Inc. and its affiliates); the
license header is kept in both upstream files. The files are shipped as opaque
browser assets, not linked into Rust code.

Served concatenated as `GET /v1/runtime/react-18.3.1.js` with
`Cache-Control: public, max-age=31536000, immutable`, so every frame of a page
references the same artifact and the browser downloads it once.

Verify the vendored bytes: `scripts/check-runtime-assets.sh`.
