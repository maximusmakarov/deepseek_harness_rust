# Conformance fixture: @kt11/dsh-deepseek-balance 0.2.1

A real, third-party DSH client plugin used to prove C-lite conformance
(`docs/adr/040-react-client-half.md`, round 79):

- `dsh/client.js` is the upstream **unmodified** client half (hand-written
  lazy-CJS module-table bundle, `require("react")` / `require("react-dom")`,
  registers `sidebar.footer.action`). SHA-256 is recorded in `package.json`
  (`dshrFixture.clientSha256`).
- `dsh/index.js` is the upstream **unmodified** host half (zero runtime deps,
  node builtins + global fetch). It registers `/dsh-balance/deepseek` and the
  `/dsh-usage/*` routes through `ctx.inject(['webServer'])`; dshr serves them via
  the C1 sidecar proxy, so the card fetches its own plugin routes instead of
  receiving a 404 page.
- `dsh.client.host_modules: ["react", "react-dom/client"]` is the only manifest
  change: it opts the frame into the shared host runtime instead of bundling
  React.

Upstream: MIT, https://www.npmjs.com/package/@kt11/dsh-deepseek-balance
(version 0.2.1). Vendored only as a test fixture, not linked into dshr.
