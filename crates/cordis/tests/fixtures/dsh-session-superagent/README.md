# Conformance fixture: dsh-session-superagent 0.10.12

Another real third-party DSH client plugin used for C-lite conformance
(`docs/adr/040-react-client-half.md`, round 82):

- `client.js` is the upstream **unmodified** client half; SHA-256 is recorded in
  `package.json` (`dshrFixture.clientSha256`).
- `stub.mjs/stub.mjs` replaces the host half. The client half under test
  degrades gracefully when its host services/routes are absent.
- Upstream `dsh.client` declarations are kept; `host_modules` is the only
  addition.

Upstream: MIT, https://www.npmjs.com/package/dsh-session-superagent.
Vendored only as a test fixture, not linked into dshr.
