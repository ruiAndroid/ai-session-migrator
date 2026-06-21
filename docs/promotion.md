# Promotion Copy

## Short Description

Local-first desktop app for migrating Codex sessions between AI providers, with preview, backup, and archived-session cleanup.

## GitHub About

**Description**

Local-first desktop tool for migrating Codex sessions between AI providers.

**Topics**

```text
codex
codex-desktop
ai-tools
session-migration
provider-migration
tauri
react
typescript
rust
desktop-app
windows
local-first
```

## Social Post

I built AI Session Migrator, a local-first desktop app for Codex users who switch AI providers.

It scans local sessions, lets you choose source and target providers, previews the migration, creates backups, and can clean up archived sessions. No telemetry, no cloud upload.

Windows build:

```text
https://github.com/ruiAndroid/ai-session-migrator/releases
```

## Longer Launch Post

AI Session Migrator is a small desktop tool for a very specific pain point: provider switching.

If you use Codex Desktop and have sessions tied to an older provider, this app helps you migrate selected sessions to your current provider. It scans local active and archived sessions, shows provider counts, previews changes before writing, creates backups, and supports archived-session cleanup.

The important part: it is local-first. Session files can contain private prompts, code, local paths, and business context, so the app does not upload or remotely parse session data.

Built with Tauri, React, TypeScript, and Rust.

GitHub:

```text
https://github.com/ruiAndroid/ai-session-migrator
```

## Release Page Checklist

- Add a screenshot of the main session list.
- Add a screenshot of migration preview.
- Add a screenshot of completion notice with backup actions.
- Upload `AI-Session-Migrator-Windows-x64.exe`.
- Mention Windows SmartScreen warning for unsigned early builds.
- Mention local-only safety model.
