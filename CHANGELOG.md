# Changelog

## 0.1.0 — current

- M0–M2: workspace, config, LLM adapters, JSONL sessions, tools, agent loop, TUI.
- M3: prompt/AGENTS, skills, compaction, fork/rewind/title/search.
- M4: MCP stdio, web tools, jobs, subagents.
- M5: Seatbelt/Landlock sandbox, MCP plugin manager.
- M6: HTTP/SSE server, Web UI, JSON-RPC.
- M7: workflows, teams (durable), worktrees, schedules, LSP, Office, CUA, FTS5, WASM, OTel, ACP.
- Hardening: Linux seccomp/network, tool allowlist, agent-tree budget, scheduler lock.
- Packaging: Dockerfile, release workflow, install script.
- Plugins: Cordis hot reload (supervisor, file/config watcher, `/v1/plugins/reload`)
  and the client half (`dsh.client` served into a sandboxed iframe with a
  `postMessage` bridge and a Web UI panel).

Known gaps: Windows AppContainer is experimental/opt-in; desktop Tauri and full i18n are deferred.
