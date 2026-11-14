# Conformance fixture: dsh-remote 1.1.2

Second real third-party DSH client plugin used for C-lite conformance
(`docs/adr/040-react-client-half.md`, round 80):

- `lib/client.js` is the upstream **unmodified** client half (lazy-CJS module
  table, requires `react`, `react-dom`, `react/jsx-runtime` and
  `@deepseek-ai/dsh-client-ui-primitives`; registers
  `conversation.session.header.actions`, `conversation.session.header.utilities`,
  `sidebar.workspaces.directoryFlow` and more). SHA-256 is recorded in
  `package.json` (`dshrFixture.clientSha256`).
- `lib/stub.mjs` replaces the host half (upstream opens an SSH/IDE bridge that
  needs the DSH web server and terminal services). The client half under test
  degrades gracefully when those services are absent.
- `dsh.client.host_modules` is the only manifest change: it opts into the shared
  host runtime + partial primitives shim.

Upstream: MIT, https://www.npmjs.com/package/dsh-remote (version 1.1.2).
Vendored only as a test fixture, not linked into dshr.
