# Conformance fixture: dsh-plugin-studio 0.3.1

Another real third-party DSH client plugin used for C-lite/C2 conformance
(`docs/adr/040-react-client-half.md`, round 85):

- `lib/client.js` is the upstream **unmodified** client half; SHA-256 is recorded in
  `package.json` (`dshrFixture.clientSha256`).
- `lib/stub.mjs/stub.mjs` replaces the host half (it needs DSH plugin-manager/
  settings services); the client half activates cleanly and leaves
  host-service-dependent UI unmounted — the graceful-degradation case.
- `dsh.client.host_modules` is the only client-declaration addition.

Upstream: MIT, https://www.npmjs.com/package/dsh-plugin-studio.
Vendored only as a test fixture, not linked into dshr.
