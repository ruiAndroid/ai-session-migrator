# AI Session Migrator v0.1.1

Patch release focused on making Codex Desktop session repair more reliable.

## What's New

- Added a dedicated session repair workspace for Codex visibility issues.
- Repairs Codex catalog, `session_index.jsonl`, and both known state databases.
- Detects and fixes stale `cwd` / `rollout_path` metadata, including Windows extended paths such as `\\?\D:\...`.
- Adds a compatibility repair for rollouts containing `internal_chat_message_metadata_passthrough`, which older bundled Codex Desktop backends can reject.
- Shows separate repair counts for catalog, session index, and state metadata so users can see why a session is still invisible.
- Keeps migration and repair flows separated into tabs so normal provider migration stays uncluttered.

## Download

Download:

```text
AI-Session-Migrator-Windows-x64-setup.exe
AI-Session-Migrator-macOS-universal-unsigned.dmg
```

## Notes

- Close Codex / Codex Desktop before applying repairs, because the app writes Codex visibility metadata after creating backups.
- Repair keeps JSONL transcripts in place. The compatibility repair only removes the known hidden passthrough field after backup.
- On Windows, use the setup installer artifact. Do not launch or redistribute a raw Cargo-built `target/release/ai-session-migrator.exe`.
- The macOS DMG is currently unsigned.
- The app processes data locally and does not upload session files.

## Verification

This release was verified with:

```powershell
git diff --check
npm test
npm run web:build
cargo test --manifest-path apps\desktop\src-tauri\Cargo.toml --features desktop
```
