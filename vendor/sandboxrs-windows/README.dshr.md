# dshr fork of sandboxrs-windows 0.1.1

Only change: `apply_baseline_readonly` is skipped during AppContainer setup.
The upstream function recursively grants read/execute over `SystemRoot`,
`ProgramFiles`, and related roots for every command; on GitHub Windows this
exceeded the 60s test timeout. AppContainer already reads system binaries via
default ACLs. Workspace/temp policy grants are unchanged.

Upstream: https://github.com/TarunKurella/sandboxrs-windows (MIT).

Also skips the backend probe in `backend::select`; the probe launched a
throwaway AppContainer for every command and dominated startup. AppContainer
setup still fails closed if the profile/ACL cannot be created.
