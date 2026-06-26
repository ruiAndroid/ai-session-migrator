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

- 2026-06-26: `AGENTS.md` and `MEMORY.md` were missing in this subproject, and `~/.codex/templates` was also missing, so minimal inferred files were created.
- 2026-06-26: Windows user-facing releases must publish the Tauri NSIS setup installer, not the raw Cargo-built `target/release/ai-session-migrator.exe`; the raw exe can load the dev URL (`127.0.0.1:5173`) when frontend resources are not bundled for distribution.
- 2026-06-26: Tauri bundle targets are intentionally limited to `["nsis"]` on Windows to avoid unnecessary WiX/MSI downloads during local and CI packaging. macOS release workflow passes `--bundles dmg` explicitly.
- 2026-06-26: Codex user-renamed session titles are stored in visibility metadata such as `session_index.jsonl.thread_name` and SQLite `threads.title`, while the session JSONL can still contain the old/generated title. Provider migration must preserve the user-visible title instead of overwriting SQLite title from JSONL-derived metadata.
- 2026-06-26: Because the desktop app now minimizes to tray on close, local release builds can fail to overwrite `target/release/ai-session-migrator.exe` if the app is still running in the tray. Exit the app from the tray or stop the process before rebuilding.
