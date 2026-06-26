# AI Session Migrator

[简体中文](README.md) | English

![AI Session Migrator product preview](assets/github-social-preview.png)

**AI Session Migrator** is a local-first desktop app for migrating Codex sessions between AI providers.

It helps you scan local Codex Desktop sessions, choose a source provider, preview the change, create backups, and migrate selected sessions to your target provider without uploading your conversation data anywhere.

> Windows and macOS desktop app. Built with Tauri, React, TypeScript, and Rust.

## Why This Exists

When you switch AI providers, old coding sessions can remain tied to the previous `model_provider`. That can make sessions harder to continue, organize, or repair.

AI Session Migrator gives you a safer desktop workflow:

- Scan active and archived Codex sessions.
- See which provider each session currently uses.
- Select the source provider and target provider.
- Preview changes before writing files.
- Back up affected session data before migration or deletion.
- Delete archived sessions when you want to clean up old history.

## Highlights

- **Desktop-first workflow**: no browser tab required for normal use.
- **Local-only processing**: no telemetry, no cloud upload, no remote parsing.
- **Provider migration**: migrate selected sessions from one provider to another.
- **Target provider picker**: choose discovered providers or type a custom provider.
- **Active/archived session awareness**: active sessions are shown first; archived sessions are labeled.
- **Archived cleanup**: delete selected archived sessions after confirmation and backup.
- **Backups before writes**: migration and deletion create backup directories first.
- **Preview and confirmation**: review planned changes before applying them.
- **Chinese UI today**: the current desktop interface is optimized for Chinese users.

## Download

The easiest way to distribute builds is through GitHub Releases.

1. Open the repository's **Releases** page.
2. On Windows, download `AI-Session-Migrator-Windows-x64-setup.exe`.
3. On macOS, download `AI-Session-Migrator-macOS-universal-unsigned.dmg`.
4. Run the app and click **扫描会话**.

The current macOS build is an unsigned preview DMG. If macOS says the developer cannot be verified, right-click the app in Finder and choose **Open**, or allow it from **System Settings > Privacy & Security**.

Windows may show a SmartScreen warning for early unsigned builds. This is expected until the project has a signed installer or broader reputation.

The app reads the current user's `.codex` directory by default. You can also point it to another Codex data directory manually.

## Safety Model

Session files can contain private prompts, code, local paths, and business context. This project treats those files as sensitive.

The safety rules are simple:

1. **No telemetry**
2. **No cloud upload**
3. **Backup before write**
4. **Confirm before destructive actions**

Backups are created under the selected Codex home before migration or archived-session deletion.

## Screenshots

Screenshots will be added after the first public release build.

Recommended screenshots for the GitHub page:

- Main scan result with provider dropdowns.
- Migration preview.
- Migration completion notice with backup actions.
- Archived-session deletion confirmation.

## Development

Install dependencies:

```powershell
npm install
```

Run the desktop app:

```powershell
npm run dev
```

For frontend-only debugging:

```powershell
npm run web:dev
```

Build the development verification executable for the current system:

```powershell
npm run build
```

The Windows development verification executable is written to:

```text
apps/desktop/src-tauri/target/release/ai-session-migrator.exe
```

Build distributable installer bundles for the current system:

```powershell
npm --workspace apps/desktop run desktop:bundle
```

Installer bundling may download external packaging tools such as NSIS/WiX on Windows. For user-facing releases, publish the installer artifact rather than `target/release/ai-session-migrator.exe`.

macOS DMG builds must run on macOS, either locally on a Mac or through the GitHub Actions macOS runner.

## Verification

Run frontend tests:

```powershell
npm test
```

Run Rust core tests:

```powershell
cd apps/desktop/src-tauri
cargo test --lib
```

Run a desktop installer build:

```powershell
npm run desktop:bundle
```

On Windows, the desktop scripts automatically load the Visual Studio C++ environment when Visual Studio or Build Tools are installed. Desktop builds also require a Windows SDK component because Rust/Tauri links against Windows system libraries such as `kernel32.lib`.

## Release Automation

This repository includes GitHub Actions workflows that build a Windows executable and an unsigned macOS DMG when a version tag is pushed:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

The workflows upload these artifacts to the GitHub Release:

- `AI-Session-Migrator-Windows-x64-setup.exe`
- `AI-Session-Migrator-macOS-universal-unsigned.dmg`

## Roadmap

- Add English UI mode.
- Add signed Windows installer.
- Add signed and notarized macOS DMG.
- Add richer session search and filters.
- Add safer restore-from-backup workflow.
- Expand provider migration checks as Codex storage formats evolve.

## License

MIT
