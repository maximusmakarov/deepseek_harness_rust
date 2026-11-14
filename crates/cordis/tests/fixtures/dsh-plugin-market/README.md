# Conformance fixture: @99galaxy/dsh-plugin-market 1.2.2

Another real third-party DSH client plugin used for C-lite conformance
(`docs/adr/040-react-client-half.md`, round 83):

- `client/client.js` is the upstream **unmodified** client half; SHA-256 is recorded in
  `package.json` (`dshrFixture.clientSha256`).
- `client/stub.mjs/stub.mjs` replaces the host half; the client half degrades
  gracefully when its host services/routes are absent.
- Upstream `dsh.client` declarations are kept; `host_modules` is the only
  addition.

Upstream: MIT, https://www.npmjs.com/package/@99galaxy/dsh-plugin-market.
Vendored only as a test fixture, not linked into dshr.
