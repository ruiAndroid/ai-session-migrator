# MEMORY.md

## Project Facts

- Project name: `ai-session-migrator`.
- Workspace root: `D:\dev\AI\AIPro\fun-claw\ai-session-migrator`.
- The product goal is a desktop app for managing and migrating Codex sessions.
- The desktop app uses Tauri 2, Rust, React, TypeScript, and Vite.

## User Preferences

- User name: rui.
- Default communication language is Chinese.
- Prioritize the desktop app path; avoid browser-based detours for this project.
- Explain decisions by "why" and "impact on users".

## Session Notes

- 2026-07-06: Codex 可见索引修复已落地为独立于 provider 迁移的桌面流程：扫描后展示 `~/.codex/sqlite/codex-dev.db.local_thread_catalog` 缺失项，默认只选择活跃且缺失 catalog 的会话，归档会话可见但不默认选中；预览显示将插入的 catalog 标题/cwd/source/provider，确认修复前必须已预览同一选择。apply 后端仍负责检测 Codex/Codex Desktop 进程、事务写入 SQLite、备份 `codex-dev.db`/WAL/SHM、state DB 和 `session_index.jsonl`，且不移动、删除或改写 JSONL。

- 2026-07-06: Codex Desktop 左侧项目会话列表可能与真实会话文件不一致，尤其在清理工具误删/影响本地 cache 后。ai-session-migrator 的后续修复能力应按三层模型处理：真实会话源是 `~/.codex/sessions/**/*.jsonl` / `archived_sessions`，旧/主线程状态在 `~/.codex/state_5.sqlite` 与 `~/.codex/sqlite/state_5.sqlite` 的 `threads`，Codex Desktop 左栏可见 catalog 在 `~/.codex/sqlite/codex-dev.db.local_thread_catalog`。修复方向固定为新增“修复 Codex 可见索引”：默认只补齐缺失 catalog 记录，不移动/删除/改写 JSONL，不清空重建 catalog；apply 前必须检测 Codex 进程，运行中拒绝写入，并备份相关 sqlite/WAL/SHM 与 `session_index.jsonl`。

- 2026-06-26: `AGENTS.md` and `MEMORY.md` were missing in this subproject, and `~/.codex/templates` was also missing, so minimal inferred files were created.
- 2026-06-26: Windows user-facing releases must publish the Tauri NSIS setup installer, not the raw Cargo-built `target/release/ai-session-migrator.exe`; the raw exe can load the dev URL (`127.0.0.1:5173`) when frontend resources are not bundled for distribution.
- 2026-06-26: Tauri bundle targets are intentionally limited to `["nsis"]` on Windows to avoid unnecessary WiX/MSI downloads during local and CI packaging. macOS release workflow passes `--bundles dmg` explicitly.
- 2026-06-26: Codex user-renamed session titles are stored in visibility metadata such as `session_index.jsonl.thread_name` and SQLite `threads.title`, while the session JSONL can still contain the old/generated title. Provider migration must preserve the user-visible title instead of overwriting SQLite title from JSONL-derived metadata.
- 2026-06-26: Because the desktop app now minimizes to tray on close, local release builds can fail to overwrite `target/release/ai-session-migrator.exe` if the app is still running in the tray. Exit the app from the tray or stop the process before rebuilding.
- 2026-06-30: Archived session rows should be visually de-emphasized through row/content styling, not whole-row opacity, because archive/activate/delete actions must still look and behave clickable.
- 2026-07-02: Session project ownership is derived from each Codex session's `session_meta.payload.cwd`. The desktop scan response exposes a short `projectName` for list badges and `projectPath` for advanced details; if `cwd` is missing or empty, the list should omit the project badge instead of showing a misleading unknown project.
