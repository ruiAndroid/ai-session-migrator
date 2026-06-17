# AI Session Migrator

AI Session Migrator is a local-first desktop tool for moving AI coding assistant sessions from one provider to another.

The first supported target is Codex Desktop. The app is designed for provider-switching workflows such as moving selected sessions from an old provider to the currently configured provider.

## Goals

- Show local sessions in a beginner-friendly desktop interface.
- Let users choose a source provider, target provider, and sessions to migrate.
- Preview changes before writing.
- Create backups before applying any migration.
- Keep all session data local.

## Safety

Session files can include private prompts, code, paths, and business context.

This project follows three rules:

1. No telemetry.
2. No cloud upload.
3. Backups before writes.

## Development

Desktop app:

```powershell
npm install
npm run dev
```

Run frontend tests:

```powershell
npm test
```

## Prototype

The earlier Python prototype lives outside this repository for now in `../ai-session-doctor`. It is used as behavior reference while the Rust core is built.
