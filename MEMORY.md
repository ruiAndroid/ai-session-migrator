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

- 2026-07-07: `apps/desktop/src/styles.test.ts` 这类直接读取 CSS 文本的测试在 Windows 工作区会遇到 CRLF 行尾；如果 selector 断言包含换行，先把读取内容标准化为 LF，避免把真实样式存在误判成规则缺失。

- 2026-07-06: `rollout ... does not start with session metadata` 不能只按 state/catalog 路径修。对 `019f32e8-178a-7b01-9a43-61e5a75d73ae` 做临时 Codex Home 对照时，`session_meta` 首行合法、最短 prefix 可被旧后端读取，但完整 `cli_version=0.142.5` rollout 会被 bundled `codex-cli 0.140.0-alpha.2` 拒绝；删除 JSONL 各行 `payload.internal_chat_message_metadata_passthrough` 后同一会话可进入 `thread.started`。修复器新增 `rollout_internal_metadata_passthrough`：扫描显示“会话兼容性待修复”，apply 在备份 JSONL 后只剥离该隐藏字段，保留正文、工具调用、时间戳、provider 和可见性元数据；真实写入仍要求 Codex 已退出。

- 2026-07-06: Codex Desktop 报 `rollout ... does not start with session metadata` 时，如果 JSONL 首行已经是合法 `session_meta`，优先检查 `~/.codex/state_5.sqlite` 和 `~/.codex/sqlite/state_5.sqlite` 两处 `threads.cwd/rollout_path` 是否残留 Windows extended path；修复 UI 必须单独展示 `state 待修复` 计数，避免用户只看到 catalog 已补齐而误以为全部修完。

- 2026-07-06: Codex Desktop 报 `rollout ... does not start with session metadata` 不一定表示 JSONL 损坏；本次 `slack1` 复现中 JSONL 首行是合法 `session_meta`，真正问题是 `~/.codex/state_5.sqlite` / `~/.codex/sqlite/state_5.sqlite` 的 `threads.rollout_path/cwd` 写入了 Windows extended path（`\\?\...`）。ai-session-migrator 的会话修复必须在写入和比较可见性元数据时规范化 `\\?\C:\...` -> `C:\...`、`\\?\UNC\...` -> `\\server\...`，并把已有 state 行的 `state_rollout_path_mismatch/state_cwd_mismatch` 当作可修复问题；修复仍需在 Codex 退出后执行并先备份。

- 2026-07-06: Codex 可见索引修复不能只补 `local_thread_catalog`。本机复现显示已有 catalog 行的会话仍可能因为缺 `session_index.jsonl` 记录或缺 `~/.codex/state_5.sqlite` / `~/.codex/sqlite/state_5.sqlite` 的 `threads` 行而不在 Codex Desktop 左栏正确出现。修复流程应扫描并默认选择活跃会话中的 `missing_catalog_entry`、`catalog_cwd_mismatch`、`catalog_title_stale`、`missing_session_index`、`missing_state_entry`；apply 必须先备份，再补 catalog、追加缺失的 active `session_index` 行、按会话原 provider upsert 两个现存 state DB，且始终不移动/删除/改写 JSONL transcript。

- 2026-07-06: Codex 可见索引修复已落地为独立于 provider 迁移的桌面流程：扫描后展示 `~/.codex/sqlite/codex-dev.db.local_thread_catalog` 缺失项，默认只选择活跃且缺失 catalog 的会话，归档会话可见但不默认选中；预览显示将插入的 catalog 标题/cwd/source/provider，确认修复前必须已预览同一选择。apply 后端仍负责检测 Codex/Codex Desktop 进程、事务写入 SQLite、备份 `codex-dev.db`/WAL/SHM、state DB 和 `session_index.jsonl`，且不移动、删除或改写 JSONL。

- 2026-07-06: Codex Desktop 左侧项目会话列表可能与真实会话文件不一致，尤其在清理工具误删/影响本地 cache 后。ai-session-migrator 的后续修复能力应按三层模型处理：真实会话源是 `~/.codex/sessions/**/*.jsonl` / `archived_sessions`，旧/主线程状态在 `~/.codex/state_5.sqlite` 与 `~/.codex/sqlite/state_5.sqlite` 的 `threads`，Codex Desktop 左栏可见 catalog 在 `~/.codex/sqlite/codex-dev.db.local_thread_catalog`。修复方向固定为新增“修复 Codex 可见索引”：默认只补齐缺失 catalog 记录，不移动/删除/改写 JSONL，不清空重建 catalog；apply 前必须检测 Codex 进程，运行中拒绝写入，并备份相关 sqlite/WAL/SHM 与 `session_index.jsonl`。

- 2026-06-26: `AGENTS.md` and `MEMORY.md` were missing in this subproject, and `~/.codex/templates` was also missing, so minimal inferred files were created.
- 2026-06-26: Windows user-facing releases must publish the Tauri NSIS setup installer, not the raw Cargo-built `target/release/ai-session-migrator.exe`; the raw exe can load the dev URL (`127.0.0.1:5173`) when frontend resources are not bundled for distribution.
- 2026-06-26: Tauri bundle targets are intentionally limited to `["nsis"]` on Windows to avoid unnecessary WiX/MSI downloads during local and CI packaging. macOS release workflow passes `--bundles dmg` explicitly.
- 2026-06-26: Codex user-renamed session titles are stored in visibility metadata such as `session_index.jsonl.thread_name` and SQLite `threads.title`, while the session JSONL can still contain the old/generated title. Provider migration must preserve the user-visible title instead of overwriting SQLite title from JSONL-derived metadata.
- 2026-06-26: Because the desktop app now minimizes to tray on close, local release builds can fail to overwrite `target/release/ai-session-migrator.exe` if the app is still running in the tray. Exit the app from the tray or stop the process before rebuilding.
- 2026-06-30: Archived session rows should be visually de-emphasized through row/content styling, not whole-row opacity, because archive/activate/delete actions must still look and behave clickable.
- 2026-07-02: Session project ownership is derived from each Codex session's `session_meta.payload.cwd`. The desktop scan response exposes a short `projectName` for list badges and `projectPath` for advanced details; if `cwd` is missing or empty, the list should omit the project badge instead of showing a misleading unknown project.
- 2026-07-06: ai-session-migrator 主工作区采用 `会话迁移` / `会话修复` 双 Tab，默认停留在迁移页；会话修复页的标题为“修复 Codex 会话可见性”，底层仍处理 catalog、session_index 和 state DB 可见性元数据，避免修复面板和问题列表挤压迁移会话列表。
