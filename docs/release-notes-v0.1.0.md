# AI Session Migrator v0.1.0

First public desktop build.

## What It Does

AI Session Migrator is a local-first Windows and macOS desktop app for migrating Codex sessions between AI providers.

Use it when you have old Codex sessions tied to a previous provider and want to move selected sessions to your current provider with preview, confirmation, and automatic backups.

## Included In This Release

- Scan local Codex sessions from the selected `.codex` directory.
- Detect active and archived sessions.
- Show active sessions before archived sessions.
- Filter by source provider.
- Choose a target provider from a dropdown or type a custom provider.
- Preview migration before writing files.
- Confirm migration before applying changes.
- Create backups before migration.
- Delete selected archived sessions after confirmation and backup.
- Copy backup path or open the backup directory after completion.

## Download

Download:

```text
AI-Session-Migrator-Windows-x64.exe
AI-Session-Migrator-macOS-universal-unsigned.dmg
```

## Notes

- The macOS DMG is currently unsigned. If macOS blocks the first launch, right-click the app and choose **Open**, or allow it from **System Settings > Privacy & Security**.
- The UI is currently Chinese.
- The app processes data locally and does not upload session files.
- Early unsigned builds may trigger Windows SmartScreen warnings.

## Verification

This release was verified with:

```powershell
npm test
npm --workspace apps/desktop run build
cd apps/desktop/src-tauri
cargo test --lib
```
