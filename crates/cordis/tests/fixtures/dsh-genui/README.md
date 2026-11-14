# Conformance fixture: genui 0.3.3

Another real third-party DSH client plugin used for C-lite conformance
(`docs/adr/040-react-client-half.md`, round 81):

- `lib/client.js` is the upstream **unmodified** client half; SHA-256 is recorded in
  `package.json` (`dshrFixture.clientSha256`).
- `lib/stub.mjs` replaces the host half (it needs
  DSH host services/routes). The client half under test degrades gracefully when
  its host endpoints are absent.
- `dsh.client.host_modules` is the only manifest change: it opts into the shared
  host runtime (and the partial primitives shim where used).

Upstream: MIT, https://www.npmjs.com/package/dsh-genui.
Vendored only as a test fixture, not linked into dshr.
