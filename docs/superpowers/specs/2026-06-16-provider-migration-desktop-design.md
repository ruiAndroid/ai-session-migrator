# AI Session Migrator Provider Migration Desktop Design

Date: 2026-06-16

## Goal

Build the first real desktop version of AI Session Migrator as a narrow Codex Desktop provider migration workflow.

The app should let a user scan a local Codex home, choose a source provider, choose a target provider, select sessions, preview the migration, and then apply it with backups. This version should not become a full session repair console. It should focus on making the provider migration path reliable, understandable, and safe.

## Current Context

`../ai-session-doctor` is the behavior reference. It already implements the important local-first repair behavior in Python:

- Scan Codex `sessions/**/rollout-*.jsonl` files.
- Parse session metadata and first user preview text.
- Detect UTF-8 BOM, provider mismatch, missing `session_index.jsonl` rows, missing sqlite thread rows, sqlite provider mismatch, and archived sqlite rows.
- Migrate `model_provider` with byte-oriented replacement.
- Create a backup directory before writes.
- Rebuild visibility metadata in `session_index.jsonl`, `state_5.sqlite`, and `sqlite/state_5.sqlite`.
- Produce a UI-friendly dashboard model.

`ai-session-migrator` is the final desktop product. It currently has a Tauri + React shell with mock session data. The next milestone should replace the mock data with real Tauri commands and Rust migration logic.

## Product Scope

In scope:

- Codex Desktop local session provider migration.
- Default Codex home detection using the current user's `.codex` directory.
- Manual Codex home path input for non-default stores.
- Scan before migration.
- Source provider filtering.
- Target provider dropdown with discovered and custom options.
- Session selection.
- Dry-run preview.
- Confirmed apply with backups.
- Result display including changed thread IDs and backup path.
- Migration-time visibility metadata repair for selected sessions.

Out of scope for this milestone:

- Full repair console for arbitrary session issues.
- Multi-assistant support beyond Codex Desktop.
- Cloud sync, telemetry, or remote upload.
- Editing session content beyond the minimal provider marker replacement.
- Rewriting the Streamlit UI.
- Runtime dependency on Python.

## Chosen Approach

Port the `ai-session-doctor` core behavior into Rust and expose it through Tauri commands.

This is preferred over calling the Python prototype because the desktop app should be easy to package, should not depend on the user's Python environment, and should have a stable typed command boundary between Rust and React.

## User Flow

1. User opens the desktop app.
2. App pre-fills the Codex home path with the current user's `.codex`.
3. User clicks scan.
4. Rust scans local sessions and returns a dashboard model plus provider options.
5. User chooses a source provider:
   - Default: all migratable source providers.
   - Optional: a specific discovered source provider.
6. User chooses a target provider:
   - Current `config.toml` provider appears first and is marked recommended.
   - Providers discovered from session files appear next.
   - A custom provider option reveals a text input.
7. UI shows filtered sessions with issue summary, title, short ID, current provider, updated time, and advanced details.
8. User selects sessions.
9. User clicks preview.
10. Rust returns the exact sessions that would change and metadata repairs that would happen.
11. User clicks confirm migration.
12. Rust creates a backup directory, writes selected changes, updates index/sqlite visibility metadata, and returns a result.
13. UI shows success, changed thread IDs, and backup path.

## Tauri Command Contract

### `scan_codex_home`

Input:

- `codexHome: string`

Output:

- `dashboard: DashboardModel`
- `providerOptions: ProviderOptions`
- `configProvider: string | null`

Behavior:

- Reads local files only.
- Does not write anything.
- Aggregates source providers found in session files.
- Includes the current configured provider from `config.toml` when present.

### `preview_provider_migration`

Input:

- `codexHome: string`
- `sourceProvider: string | null`
- `targetProvider: string`
- `threadIds: string[]`

Output:

- `changedThreads: string[]`
- `plannedRepairs: PlannedRepair[]`
- `dryRun: true`

Behavior:

- Does not write anything.
- Validates that target provider is not empty.
- Filters selected sessions by source provider when provided.
- Reports planned provider changes and visibility metadata repairs.

### `apply_provider_migration`

Input:

- `codexHome: string`
- `sourceProvider: string | null`
- `targetProvider: string`
- `threadIds: string[]`

Output:

- `changedThreads: string[]`
- `backupDir: string`
- `dryRun: false`

Behavior:

- Requires non-empty target provider and selected threads.
- Creates one backup directory before writing.
- Removes BOM from affected JSONL files.
- Replaces only the first compact or spaced `model_provider` JSON field marker.
- Ensures selected sessions are present in `session_index.jsonl`.
- Upserts or updates matching rows in `state_5.sqlite` and `sqlite/state_5.sqlite`.
- Clears archived state for selected rows.

## Shared Data Shapes

The React types should mirror the Rust response shape rather than invent a separate mock model.

`DashboardModel`:

- `codexHome: string`
- `totalThreads: number`
- `problemThreads: number`
- `issueCounts: Record<string, number>`
- `rows: ThreadRow[]`

`ThreadRow`:

- `threadId: string`
- `shortId: string`
- `displayName: string`
- `path: string`
- `fileProvider: string | null`
- `configProvider: string | null`
- `issueCodes: string[]`
- `severity: number`
- `canMigrate: boolean`
- `suggestedActionCode: string`
- `suggestedActionValues: Record<string, string>`
- `updatedAtMs: number`

`ProviderOptions`:

- `currentConfigProvider: string | null`
- `sourceProviders: string[]`
- `targetProviders: ProviderOption[]`

`ProviderOption`:

- `value: string`
- `label: string`
- `kind: "config" | "discovered"`
- `recommended: boolean`

The UI also supports a local-only `"custom"` target provider state that resolves to the user's custom string before invoking Rust.

## Rust Core Design

Add focused Rust modules under `apps/desktop/src-tauri/src/`:

- `main.rs`: Tauri bootstrap and command registration.
- `codex/mod.rs`: public command-facing functions and shared structs.
- `codex/scan.rs`: scan session files, config, index, and sqlite state entries.
- `codex/metadata.rs`: parse session metadata from JSONL bytes.
- `codex/migration.rs`: preview and apply provider migration.
- `codex/sqlite.rs`: read and update Codex sqlite thread rows.
- `codex/backup.rs`: create backup directories and copy affected files.

Important implementation details:

- Keep provider replacement byte-oriented to avoid corrupting non-ASCII session content.
- Strip a UTF-8 BOM only for affected files during apply.
- Decode JSONL as UTF-8 for metadata parsing after BOM removal.
- Use `serde_json` for row parsing once bytes are decoded.
- Use `rusqlite` for sqlite operations.
- Open sqlite read-only during scan.
- Use short busy timeouts for apply to avoid hanging indefinitely if Codex is using the database.

## UI Design

The desktop UI should stay a work-focused tool surface:

- Left setup column:
  - Codex home path.
  - Scan button.
  - Source provider dropdown.
  - Target provider dropdown.
  - Custom target provider input when needed.
- Main work area:
  - Scan summary metrics.
  - Migration preview summary.
  - Session table/list with checkboxes.
  - Advanced details disclosure per row.
  - Preview and confirm buttons.
  - Result panel after apply.

The UI should remove the current mojibake strings and use real Chinese copy. English labels can remain for technical identifiers like `model_provider`, `dry-run`, and provider names.

## Error Handling

Rust commands should return structured errors with a short code and user-facing message.

Required cases:

- Codex home does not exist.
- `sessions` directory does not exist.
- No sessions found.
- Target provider is empty.
- No sessions selected.
- Selected thread no longer exists.
- JSONL cannot be parsed as expected.
- Session metadata is missing an ID.
- SQLite is unavailable or locked during apply.
- Backup directory cannot be created.
- File write fails.

Apply should fail before writes when validation or backup creation fails. If a later write fails, the UI should show the backup path and the failed operation so the user can recover manually.

## Safety Model

- No network calls.
- No telemetry.
- No session upload.
- Dry-run never writes.
- Apply always creates backups before writing.
- Only selected sessions are modified.
- File writes should be minimal and targeted.
- The app should surface backup location after every successful apply.

## Testing Plan

Rust tests:

- Scan returns rows for valid Codex session fixtures.
- Scan aggregates source and target provider options.
- Scan detects BOM, provider mismatch, missing index, missing sqlite row, archived row.
- Preview returns changed threads and leaves files untouched.
- Apply removes BOM, changes provider, creates backup, updates index, and upserts sqlite rows.
- Apply preserves UTF-8 Chinese text.
- Apply respects selected thread IDs.
- Custom target provider works.

Frontend tests:

- Initial page renders real workflow controls.
- Scan command result populates provider dropdowns and session rows.
- Source provider filter updates visible sessions.
- Target provider dropdown marks config provider as recommended.
- Custom provider option reveals text input.
- Selected count changes when sessions are toggled.
- Preview button is disabled until target provider and selected sessions are valid.
- Apply result shows changed thread IDs and backup path.

Manual verification:

- `npm test`
- `npm run build`
- `cargo check` in `apps/desktop/src-tauri`
- Local browser smoke test of the Vite UI

## Acceptance Criteria

- The desktop app no longer depends on mock session data for the main workflow.
- A user can migrate selected Codex sessions from one provider to a chosen target provider.
- Target provider is chosen from a dropdown with current config, discovered providers, and custom input.
- Preview mode shows what would change without writing.
- Apply mode creates backups before writes and returns the backup path.
- Provider migration preserves non-ASCII session text.
- `session_index.jsonl` and sqlite visibility metadata are repaired for migrated sessions.
- Tests cover the Rust core and the React workflow.
