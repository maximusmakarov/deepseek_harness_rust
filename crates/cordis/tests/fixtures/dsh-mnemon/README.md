# Conformance fixture: dsh-mnemon 0.5.46

Another real third-party DSH client plugin used for C-lite conformance
(`docs/adr/040-react-client-half.md`, round 82):

- `lib/client.js` is the upstream **unmodified** client half; SHA-256 is recorded in
  `package.json` (`dshrFixture.clientSha256`).
- `lib/stub.mjs/stub.mjs` replaces the host half. The client half under test
  degrades gracefully when its host services/routes are absent.
- `dsh.client.host_modules` is the only manifest change.

Upstream: MIT, https://www.npmjs.com/package/dsh-mnemon.
Vendored only as a test fixture, not linked into dshr.
