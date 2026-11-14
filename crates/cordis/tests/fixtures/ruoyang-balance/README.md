# Conformance fixture: deepeek-balance 1.0.2

Another real third-party DSH client plugin used for C-lite conformance
(`docs/adr/040-react-client-half.md`, round 81):

- `client.js` is the upstream **unmodified** client half; SHA-256 is recorded in
  `package.json` (`dshrFixture.clientSha256`).
- `./stub.mjs` replaces the host half (it needs
  DSH host services/routes). The client half under test degrades gracefully when
  its host endpoints are absent.
- Upstream `dsh.client` declarations (platform/immediately/inject) are kept;
  `host_modules` is the only addition — it opts into the shared host runtime
  (and the partial primitives shim where used).

Upstream: MIT, https://www.npmjs.com/package/@ruoyang/dsh-plugin-deepseek-balance.
Vendored only as a test fixture, not linked into dshr.
