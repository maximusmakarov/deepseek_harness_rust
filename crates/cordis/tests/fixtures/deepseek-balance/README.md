# Conformance fixture: @kt11/dsh-deepseek-balance 0.2.1

A real, third-party DSH client plugin used to prove C-lite conformance
(`docs/adr/040-react-client-half.md`, round 79):

- `dsh/client.js` is the upstream **unmodified** client half (hand-written
  lazy-CJS module-table bundle, `require("react")` / `require("react-dom")`,
  registers `sidebar.footer.action`). SHA-256 is recorded in `package.json`
  (`dshrFixture.clientSha256`).
- `dsh/stub.mjs` replaces the host half because the upstream host half registers
  routes on the DSH web server, which the dshr Cordis sidecar does not expose;
  the client half under test does not depend on it (it degrades gracefully when
  its fetches 404).
- `dsh.client.host_modules: ["react", "react-dom/client"]` is the only manifest
  change: it opts the frame into the shared host runtime instead of bundling
  React.

Upstream: MIT, https://www.npmjs.com/package/@kt11/dsh-deepseek-balance
(version 0.2.1). Vendored only as a test fixture, not linked into dshr.
