# Codex Visible Index Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a safe "repair Codex visible index" workflow that restores missing Codex Desktop `local_thread_catalog` rows from real session files without changing JSONL transcripts.

**Architecture:** Add a focused Rust module `codex/catalog_repair.rs` that scans JSONL/state/catalog data, previews catalog inserts, and applies them in a transaction after process and backup guards. Expose three Tauri commands and a small frontend workflow that reuses the existing scan list styling while keeping provider migration separate.

**Tech Stack:** Tauri 2, Rust, rusqlite, React, TypeScript, Vitest.

---

### Task 1: Rust Catalog Scan Model

**Files:**
- Create: `apps/desktop/src-tauri/src/codex/catalog_repair.rs`
- Modify: `apps/desktop/src-tauri/src/codex/mod.rs`
- Test: `apps/desktop/src-tauri/src/codex/catalog_repair.rs`

- [ ] **Step 1: Write the failing scan test**

Add this test module skeleton to `catalog_repair.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::test_support::{init_state_db, insert_state_row_with_title, write_jsonl};
    use rusqlite::Connection;
    use std::fs;

    fn init_catalog_db(path: &std::path::Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let connection = Connection::open(path).unwrap();
        connection.execute_batch(
            "
            create table local_thread_catalog_hosts (host_id text primary key, host_kind text);
            create table local_thread_catalog (
                host_id text,
                thread_id text,
                display_title text,
                source_created_at real,
                source_updated_at real,
                cwd text,
                source_kind text,
                source_detail text,
                model_provider text,
                git_branch text,
                observation_sequence integer,
                missing_candidate integer
            );
            create table local_thread_catalog_sync_state (
                host_id text primary key,
                watermark_updated_at real,
                initial_build_complete integer,
                observation_sequence integer
            );
            ",
        ).unwrap();
    }

    #[test]
    fn scan_reports_active_session_missing_from_catalog() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let thread_id = "019f32e8-178a-7b01-9a43-61e5a75d73ae";
        write_jsonl(
            &codex.join("sessions/2026/07/05/rollout-a-019f32e8-178a-7b01-9a43-61e5a75d73ae.jsonl"),
            thread_id,
            "funai",
            false,
            "json title",
        );
        fs::write(
            codex.join("session_index.jsonl"),
            format!("{{\"id\":\"{thread_id}\",\"thread_name\":\"renamed title\"}}\n"),
        ).unwrap();
        let state_db = codex.join("state_5.sqlite");
        init_state_db(&state_db);
        insert_state_row_with_title(&state_db, thread_id, "funai", 0, "sqlite title");
        init_catalog_db(&codex.join("sqlite/codex-dev.db"));

        let response = scan_codex_catalog_repair(&codex).unwrap();

        assert_eq!(response.summary.total_threads, 1);
        assert_eq!(response.summary.missing_catalog_entries, 1);
        assert_eq!(response.rows[0].thread_id, thread_id);
        assert_eq!(response.rows[0].display_title, "renamed title");
        assert_eq!(response.rows[0].repair_codes, vec!["missing_catalog_entry"]);
        assert!(response.rows[0].selected_by_default);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test catalog_repair::tests::scan_reports_active_session_missing_from_catalog`

Expected: FAIL because `catalog_repair` module and public types/functions do not exist.

- [ ] **Step 3: Implement scan types and read-only catalog detection**

Add `pub mod catalog_repair;` to `codex/mod.rs`, define `CatalogRepairRow`, `CatalogRepairSummary`, `CatalogRepairScanResponse`, and implement `scan_codex_catalog_repair(&Path)`.

The implementation must:

```rust
pub fn scan_codex_catalog_repair(codex_home: &Path) -> Result<CatalogRepairScanResponse> {
    // validate codex_home and sessions dir
    // use scan::session_files and metadata_from_bytes
    // read session_index titles
    // read root state_5.sqlite title/provider/archive info
    // read sqlite/codex-dev.db local_thread_catalog rows
    // return missing_catalog_entry for active JSONL sessions absent from catalog
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test catalog_repair::tests::scan_reports_active_session_missing_from_catalog`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/codex/mod.rs apps/desktop/src-tauri/src/codex/catalog_repair.rs
git commit -m "feat: scan codex catalog repair state"
```

### Task 2: Preview And Apply Repair

**Files:**
- Modify: `apps/desktop/src-tauri/src/codex/catalog_repair.rs`
- Modify: `apps/desktop/src-tauri/src/codex/backup.rs` only if backup helper needs WAL/SHM coverage through explicit file list
- Test: `apps/desktop/src-tauri/src/codex/catalog_repair.rs`

- [ ] **Step 1: Write failing preview/apply tests**

Add tests:

```rust
#[test]
fn preview_returns_insert_change_without_writing_catalog() {
    // arrange one active session missing from catalog
    // call preview_codex_catalog_repair
    // assert dry_run true, planned_changes contains display_title/cwd/model_provider
    // assert local_thread_catalog row count remains 0
}

#[test]
fn apply_refuses_when_codex_process_is_running() {
    // arrange one active session missing from catalog
    // call apply_codex_catalog_repair_with_process_checker(..., || true)
    // assert error code codex_process_running
}

#[test]
fn apply_inserts_catalog_row_after_backup_and_keeps_jsonl_unchanged() {
    // arrange one active session missing from catalog
    // save original JSONL bytes
    // call apply_codex_catalog_repair_with_process_checker(..., || false)
    // assert changed_threads contains thread id
    // assert backup_dir exists
    // assert inserted catalog row has display_title, cwd, provider, missing_candidate=0
    // assert JSONL bytes unchanged
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test catalog_repair::tests::preview_returns_insert_change_without_writing_catalog catalog_repair::tests::apply_refuses_when_codex_process_is_running catalog_repair::tests::apply_inserts_catalog_row_after_backup_and_keeps_jsonl_unchanged`

Expected: FAIL because preview/apply functions do not exist.

- [ ] **Step 3: Implement preview/apply**

Implement:

```rust
pub fn preview_codex_catalog_repair(request: CatalogRepairRequest) -> Result<CatalogRepairResult>;
pub fn apply_codex_catalog_repair(request: CatalogRepairRequest) -> Result<CatalogRepairResult>;
fn apply_codex_catalog_repair_with_process_checker(
    request: CatalogRepairRequest,
    is_codex_running: impl FnOnce() -> bool,
) -> Result<CatalogRepairResult>;
```

Apply must:

- return `no_threads_selected` for empty selection.
- return `codex_process_running` when process checker says true.
- create backup before sqlite write.
- write inside a transaction.
- insert only missing catalog rows.
- keep JSONL bytes unchanged.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test catalog_repair::tests`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/codex/catalog_repair.rs
git commit -m "feat: repair codex catalog entries"
```

### Task 3: Tauri And TypeScript API

**Files:**
- Modify: `apps/desktop/src-tauri/src/main.rs`
- Modify: `apps/desktop/src-tauri/src/codex/mod.rs`
- Modify: `apps/desktop/src/domain/session.ts`
- Modify: `apps/desktop/src/domain/migrationApi.ts`
- Test: `apps/desktop/src/domain/migrationApi.ts` through existing App tests in Task 4

- [ ] **Step 1: Write failing type/API expectations in frontend tests**

In `App.test.tsx`, create a mock `catalogRepairScanResponse` and add one test that expects a visible "修复 Codex 可见索引" action after scan. This should fail because the API and UI do not exist yet.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm --workspace apps/desktop run test -- App.test.tsx -t "shows codex catalog repair action"`

Expected: FAIL because the UI action is missing.

- [ ] **Step 3: Expose commands and TypeScript types**

Add Rust command wrappers:

```rust
fn scan_codex_catalog_repair(codex_home: String) -> Result<CatalogRepairScanResponse, CommandError>;
fn preview_codex_catalog_repair(request: CatalogRepairRequest) -> Result<CatalogRepairResult, CommandError>;
fn apply_codex_catalog_repair(request: CatalogRepairRequest) -> Result<CatalogRepairResult, CommandError>;
```

Add TypeScript types:

```ts
export type CatalogRepairRow = { threadId: string; displayTitle: string; repairCodes: string[]; selectedByDefault: boolean; lifecycle: ThreadLifecycle; projectName: string | null; projectPath: string | null; };
export type CatalogRepairSummary = { totalThreads: number; missingCatalogEntries: number; selectedByDefault: number; archivedThreads: number; };
export type CatalogRepairScanResponse = { rows: CatalogRepairRow[]; summary: CatalogRepairSummary; catalogDbPath: string | null; };
export type CatalogRepairRequest = { codexHome: string; threadIds: string[]; };
export type CatalogRepairResult = { changedThreads: string[]; plannedChanges: PlannedRepair[]; backupDir: string | null; dryRun: boolean; };
```

- [ ] **Step 4: Run TypeScript check through targeted test**

Run: `npm --workspace apps/desktop run test -- App.test.tsx -t "shows codex catalog repair action"`

Expected: still FAIL until UI is implemented, but no TypeScript import/type errors.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/main.rs apps/desktop/src-tauri/src/codex/mod.rs apps/desktop/src/domain/session.ts apps/desktop/src/domain/migrationApi.ts apps/desktop/src/App.test.tsx
git commit -m "feat: expose codex catalog repair api"
```

### Task 4: Frontend Repair Workflow

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/App.test.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Write failing UI tests**

Add tests:

```ts
test("shows codex catalog repair action after scan", async () => {
  renderApp();
  await user.click(await screen.findByRole("button", { name: /扫描会话|鎵/ }));
  expect(await screen.findByRole("button", { name: /预览修复|修复 Codex 可见索引/ })).toBeEnabled();
});

test("preview and apply catalog repair use selected default rows", async () => {
  // scan returns one missing active catalog row selected by default
  // click preview repair
  // expect api.previewCodexCatalogRepair called with that thread id
  // click confirm repair
  // expect api.applyCodexCatalogRepair called with that thread id
});

test("catalog repair process-running error tells user to close Codex", async () => {
  // preview succeeds, apply rejects with code codex_process_running
  // expect actionable error text
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm --workspace apps/desktop run test -- App.test.tsx -t "catalog repair"`

Expected: FAIL because UI handlers do not exist.

- [ ] **Step 3: Implement UI**

Add state:

```ts
const [catalogRepair, setCatalogRepair] = useState<CatalogRepairScanResponse | null>(null);
const [catalogRepairSelectedIds, setCatalogRepairSelectedIds] = useState<string[]>([]);
const [catalogRepairPreview, setCatalogRepairPreview] = useState<CatalogRepairResult | null>(null);
```

After main scan, call `scanCodexCatalogRepair`. Render a compact repair panel with metrics and buttons:

- "预览修复"
- "确认修复"
- selected count
- process-running error message from `codex_process_running`

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm --workspace apps/desktop run test -- App.test.tsx -t "catalog repair"`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/App.tsx apps/desktop/src/App.test.tsx apps/desktop/src/styles.css
git commit -m "feat: add catalog repair workflow"
```

### Task 5: Verification And Build

**Files:**
- Modify only if verification exposes bugs.

- [ ] **Step 1: Run Rust tests**

Run: `cargo test` in `apps/desktop/src-tauri`.

Expected: PASS.

- [ ] **Step 2: Run frontend targeted tests**

Run: `npm --workspace apps/desktop run test -- App.test.tsx -t "catalog repair"`

Expected: PASS.

- [ ] **Step 3: Run full frontend tests**

Run: `npm test`.

Expected: PASS or report pre-existing `styles.test.ts` failure if unchanged.

- [ ] **Step 4: Build desktop app**

Run: `npm run desktop:build`.

Expected: build succeeds and produces the desktop bundle outputs.

- [ ] **Step 5: Final commit if verification fixes were needed**

```bash
git status --short
git add <changed files>
git commit -m "fix: stabilize catalog repair verification"
```

---

## Self Review

- Spec coverage: scan, preview, apply, process guard, backup, no JSONL writes, frontend workflow, and verification are covered.
- Placeholder scan: no TBD/TODO placeholders are present.
- Type consistency: Rust and TypeScript names use `CatalogRepair*` consistently.
