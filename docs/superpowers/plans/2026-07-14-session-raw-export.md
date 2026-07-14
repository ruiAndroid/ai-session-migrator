# Raw Session Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-session export action that saves the untouched Codex `.jsonl` file through the native operating-system Save As dialog.

**Architecture:** A focused Rust `codex::export` module validates that the source is a scanned session under the selected Codex Home and copies it byte-for-byte. A focused TypeScript dialog adapter owns native Save As behavior and `.jsonl` naming. `App.tsx` coordinates the two through injected interfaces so cancellation, progress, success, and failure are testable without a desktop runtime.

**Tech Stack:** Tauri 2, `tauri-plugin-dialog` 2, Rust, React 18, TypeScript, Vitest, Testing Library.

---

## File Map

- Create `apps/desktop/src-tauri/src/codex/export.rs`: raw session source validation and byte-for-byte copy.
- Modify `apps/desktop/src-tauri/src/codex/mod.rs`: export request/result types and public domain entry point.
- Modify `apps/desktop/src-tauri/src/main.rs`: async Tauri export command and dialog plugin initialization.
- Modify `apps/desktop/src-tauri/Cargo.toml` and `Cargo.lock`: add the official Tauri dialog plugin.
- Create `apps/desktop/src-tauri/capabilities/default.json`: allow only native Save dialog access for the main window.
- Modify `apps/desktop/src/domain/session.ts`: frontend export request/result types.
- Modify `apps/desktop/src/domain/migrationApi.ts`: typed `exportSession` command adapter.
- Create `apps/desktop/src/domain/migrationApi.test.ts`: verify the adapter invokes the correct Tauri command.
- Create `apps/desktop/src/domain/sessionExportDialog.ts`: native Save As adapter, default filename, and `.jsonl` enforcement.
- Create `apps/desktop/src/domain/sessionExportDialog.test.ts`: dialog cancellation and filename tests.
- Modify `apps/desktop/src/App.tsx`: row action, export coordination, progress, success, and localized errors.
- Modify `apps/desktop/src/App.test.tsx`: user-visible export workflow tests.
- Modify `apps/desktop/package.json` and root `package-lock.json`: add `@tauri-apps/plugin-dialog`.
- Modify `README.md`, `README.zh-CN.md`, and `README.en.md`: document raw session export.
- Modify `MEMORY.md`: record the durable lossless-export and source-validation contract.

### Task 1: Rust Raw Export Core

**Files:**
- Create: `apps/desktop/src-tauri/src/codex/export.rs`
- Modify: `apps/desktop/src-tauri/src/codex/mod.rs`

- [ ] **Step 1: Add request/result types and failing export tests**

Add camelCase serializable `SessionExportRequest` and `SessionExportResult` to `codex/mod.rs`, declare `pub mod export`, and add tests in `codex/export.rs` for active, archived, arbitrary-byte, outside-home, wrong-extension, missing-source, and same-source/destination behavior. The core success test must assert bytes, not parsed text:

```rust
#[test]
fn exports_active_session_byte_for_byte() {
    let temp = tempfile::tempdir().unwrap();
    let codex_home = temp.path().join(".codex");
    let source = codex_home.join("sessions/2026/07/14/rollout-a.jsonl");
    let destination = temp.path().join("exported.jsonl");
    let bytes = b"{\"type\":\"session_meta\"}\n\xFF\x00";
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, bytes).unwrap();

    let result = export_session(SessionExportRequest {
        codex_home: codex_home.display().to_string(),
        thread_id: "thread-a".to_string(),
        source_path: source.display().to_string(),
        destination_path: destination.display().to_string(),
    })
    .unwrap();

    assert_eq!(std::fs::read(destination).unwrap(), bytes);
    assert_eq!(result.thread_id, "thread-a");
    assert_eq!(result.bytes_written, bytes.len() as u64);
}
```

- [ ] **Step 2: Run the focused Rust test and verify RED**

```powershell
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml codex::export::tests --lib
```

Expected: FAIL because `export_session` is not implemented.

- [ ] **Step 3: Implement minimal validated byte copy**

Implement `export_session` in `codex/export.rs` with these stable error codes:

```rust
pub fn export_session(request: SessionExportRequest) -> Result<SessionExportResult> {
    let codex_home = canonicalize_codex_home(&request.codex_home)?;
    let source = canonicalize_source(&request.source_path)?;
    validate_source_root(&codex_home, &source)?;
    validate_jsonl(&source, "session_export_source_format_invalid")?;

    let destination = PathBuf::from(&request.destination_path);
    if !destination.is_absolute() {
        return Err(CommandError::new(
            "session_export_destination_invalid",
            "export destination must be an absolute path",
        ));
    }
    validate_jsonl(&destination, "session_export_destination_format_invalid")?;
    reject_same_file(&source, &destination)?;

    let bytes_written = std::fs::copy(&source, &destination).map_err(|error| {
        CommandError::new(
            "session_export_write_failed",
            format!("failed to export session to {}: {error}", destination.display()),
        )
    })?;

    Ok(SessionExportResult {
        thread_id: request.thread_id,
        destination_path: destination.display().to_string(),
        bytes_written,
    })
}
```

Use these helpers so missing files, path escapes, wrong formats, and self-overwrites have stable behavior:

```rust
fn canonicalize_codex_home(value: &str) -> Result<PathBuf> {
    Path::new(value).canonicalize().map_err(|error| {
        CommandError::new(
            "codex_home_missing",
            format!("Codex Home is unavailable at {value}: {error}"),
        )
    })
}

fn canonicalize_source(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    let canonical = path.canonicalize().map_err(|error| {
        let code = if error.kind() == std::io::ErrorKind::NotFound {
            "session_export_source_missing"
        } else {
            "session_export_source_invalid"
        };
        CommandError::new(code, format!("session source is unavailable at {value}: {error}"))
    })?;
    if !canonical.is_file() {
        return Err(CommandError::new(
            "session_export_source_invalid",
            format!("session source is not a file: {}", canonical.display()),
        ));
    }
    Ok(canonical)
}

fn validate_source_root(codex_home: &Path, source: &Path) -> Result<()> {
    let allowed = ["sessions", "archived_sessions"]
        .into_iter()
        .filter_map(|name| codex_home.join(name).canonicalize().ok())
        .any(|root| source.starts_with(root));
    if allowed {
        Ok(())
    } else {
        Err(CommandError::new(
            "session_export_source_outside_codex_home",
            "session source is outside the selected Codex Home",
        ))
    }
}

fn validate_jsonl(path: &Path, code: &str) -> Result<()> {
    let is_jsonl = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("jsonl"));
    is_jsonl.then_some(()).ok_or_else(|| {
        CommandError::new(code, format!("path must use the .jsonl extension: {}", path.display()))
    })
}

fn reject_same_file(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists()
        && destination
            .canonicalize()
            .is_ok_and(|canonical| canonical == source)
    {
        return Err(CommandError::new(
            "session_export_source_destination_same",
            "source and destination must be different files",
        ));
    }
    Ok(())
}
```

Canonicalization prevents a symlink from escaping the allowed roots. Do not read or parse JSONL content.

- [ ] **Step 4: Run the focused Rust tests and verify GREEN**

Run the same focused command. Expected: all `codex::export::tests` pass.

- [ ] **Step 5: Commit the Rust domain core**

```powershell
git add apps/desktop/src-tauri/src/codex/export.rs apps/desktop/src-tauri/src/codex/mod.rs
git commit -m "feat: add raw session export core"
```

### Task 2: Tauri Command And Typed API

**Files:**
- Modify: `apps/desktop/src-tauri/src/main.rs`
- Modify: `apps/desktop/src/domain/session.ts`
- Modify: `apps/desktop/src/domain/migrationApi.ts`
- Create: `apps/desktop/src/domain/migrationApi.test.ts`

- [ ] **Step 1: Write the failing TypeScript command-adapter test**

Mock `@tauri-apps/api/core` and assert the public adapter forwards the exact request:

```typescript
test("exportSession invokes the raw session export command", async () => {
  const request: SessionExportRequest = {
    codexHome: "D:\\Codex\\.codex",
    threadId: "thread-a",
    sourcePath: "D:\\Codex\\.codex\\sessions\\rollout-a.jsonl",
    destinationPath: "D:\\Exports\\rollout-a.jsonl"
  };
  await tauriMigrationApi.exportSession(request);
  expect(invoke).toHaveBeenCalledWith("export_session", { request });
});
```

- [ ] **Step 2: Run the focused Vitest test and verify RED**

```powershell
npm --workspace apps/desktop run test -- src/domain/migrationApi.test.ts
```

Expected: FAIL because `MigrationApi.exportSession` does not exist.

- [ ] **Step 3: Add the frontend contract and backend command**

Add matching TypeScript types:

```typescript
export type SessionExportRequest = {
  codexHome: string;
  threadId: string;
  sourcePath: string;
  destinationPath: string;
};

export type SessionExportResult = {
  threadId: string;
  destinationPath: string;
  bytesWritten: number;
};
```

Add `exportSession(request)` to `MigrationApi` and invoke `export_session`. In `main.rs`, add an async command using `tauri::async_runtime::spawn_blocking`, map join failures to `session_export_task_failed`, and register the command in `generate_handler!`:

```rust
#[tauri::command]
async fn export_session(
    request: SessionExportRequest,
) -> std::result::Result<SessionExportResult, CommandError> {
    tauri::async_runtime::spawn_blocking(move || codex::export_session(request))
        .await
        .map_err(|error| {
            CommandError::new(
                "session_export_task_failed",
                format!("failed to run session export: {error}"),
            )
        })?
}
```

- [ ] **Step 4: Run focused TypeScript and Rust tests**

```powershell
npm --workspace apps/desktop run test -- src/domain/migrationApi.test.ts
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml codex::export::tests --lib
```

Expected: both commands pass.

- [ ] **Step 5: Commit the command contract**

```powershell
git add apps/desktop/src-tauri/src/main.rs apps/desktop/src/domain/session.ts apps/desktop/src/domain/migrationApi.ts apps/desktop/src/domain/migrationApi.test.ts
git commit -m "feat: expose raw session export command"
```

### Task 3: Native Save As Adapter

**Files:**
- Modify: `apps/desktop/package.json`
- Modify: `package-lock.json`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/Cargo.lock`
- Create: `apps/desktop/src-tauri/capabilities/default.json`
- Modify: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src/domain/sessionExportDialog.ts`
- Create: `apps/desktop/src/domain/sessionExportDialog.test.ts`

- [ ] **Step 1: Install official dialog dependencies**

```powershell
npm install @tauri-apps/plugin-dialog@^2.0.0 --workspace apps/desktop
```

Add `tauri-plugin-dialog = "2"` to Rust dependencies. Dependency installation is setup only; do not initialize the plugin before the failing adapter test.

- [ ] **Step 2: Write failing dialog-adapter tests**

Cover Windows and Unix source paths, cancellation, missing extension, and a conflicting extension:

```typescript
test("opens Save As with the original JSONL filename", async () => {
  vi.mocked(save).mockResolvedValue("D:\\Exports\\rollout-a.jsonl");
  const result = await tauriSessionExportDialog.chooseDestination(
    "D:\\Codex\\.codex\\sessions\\rollout-a.jsonl"
  );
  expect(save).toHaveBeenCalledWith({
    defaultPath: "rollout-a.jsonl",
    filters: [{ name: "Codex 会话", extensions: ["jsonl"] }]
  });
  expect(result).toBe("D:\\Exports\\rollout-a.jsonl");
});
```

Also assert: `null` stays `null`; `D:\\Exports\\rollout-a` becomes `...rollout-a.jsonl`; and `...rollout-a.txt` rejects with `导出文件必须使用 .jsonl 后缀。`.

- [ ] **Step 3: Run dialog tests and verify RED**

```powershell
npm --workspace apps/desktop run test -- src/domain/sessionExportDialog.test.ts
```

Expected: FAIL because the adapter module does not exist.

- [ ] **Step 4: Implement the focused dialog adapter**

```typescript
export type SessionExportDialog = {
  chooseDestination(sourcePath: string): Promise<string | null>;
};

export function sessionExportFileName(sourcePath: string) {
  return sourcePath.split(/[\\/]/).filter(Boolean).at(-1) ?? "codex-session.jsonl";
}

export function normalizeJsonlDestination(path: string) {
  const fileName = sessionExportFileName(path);
  const lastDot = fileName.lastIndexOf(".");
  if (lastDot < 0) return `${path}.jsonl`;
  if (fileName.slice(lastDot).toLowerCase() === ".jsonl") return path;
  throw new Error("导出文件必须使用 .jsonl 后缀。");
}

export const tauriSessionExportDialog: SessionExportDialog = {
  async chooseDestination(sourcePath) {
    const destination = await save({
      defaultPath: sessionExportFileName(sourcePath),
      filters: [{ name: "Codex 会话", extensions: ["jsonl"] }]
    });
    return destination ? normalizeJsonlDestination(destination) : null;
  }
};
```

Initialize the Rust plugin with `.plugin(tauri_plugin_dialog::init())` and add the narrow capability:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Native Save As access for the main AI Session Migrator window.",
  "windows": ["main"],
  "permissions": ["core:default", "dialog:allow-save"]
}
```

- [ ] **Step 5: Run dialog tests and TypeScript build**

```powershell
npm --workspace apps/desktop run test -- src/domain/sessionExportDialog.test.ts
npm run web:build
```

Expected: both pass with no TypeScript errors.

- [ ] **Step 6: Commit native Save As support**

```powershell
git add package-lock.json apps/desktop/package.json apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.lock apps/desktop/src-tauri/capabilities/default.json apps/desktop/src-tauri/src/main.rs apps/desktop/src/domain/sessionExportDialog.ts apps/desktop/src/domain/sessionExportDialog.test.ts
git commit -m "feat: add native session export dialog"
```

### Task 4: Session Row Export Workflow

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/App.test.tsx`

- [ ] **Step 1: Extend test fakes and write failing workflow tests**

Add `exportSession` to `fakeApi`, inject a fake `SessionExportDialog`, and cover:

```typescript
test("exports an active session from the row action", async () => {
  const api = fakeApi();
  const exportDialog = {
    chooseDestination: vi.fn().mockResolvedValue("D:\\Exports\\rollout-a.jsonl")
  };
  const { user } = await renderWorkflow({ api, exportDialog });
  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const row = await screen.findByRole("article", { name: "活跃 provider 会话" });
  await user.click(within(row).getByRole("button", { name: "导出" }));
  expect(exportDialog.chooseDestination).toHaveBeenCalledWith(
    `${fixtureCodexHome}\\sessions\\rollout-a.jsonl`
  );
  expect(api.exportSession).toHaveBeenCalledWith({
    codexHome: fixtureCodexHome,
    threadId: activeThreadId,
    sourcePath: `${fixtureCodexHome}\\sessions\\rollout-a.jsonl`,
    destinationPath: "D:\\Exports\\rollout-a.jsonl"
  });
  expect(await screen.findByRole("status")).toHaveTextContent("D:\\Exports\\rollout-a.jsonl");
});
```

Add separate tests that archived rows also expose the action, cancellation does not call `exportSession`, a pending command shows `正在导出`, and `session_export_source_missing` is localized with a rescan instruction.

- [ ] **Step 2: Run App tests and verify RED**

```powershell
npm --workspace apps/desktop run test -- src/App.test.tsx
```

Expected: FAIL because the row has no export action and `App` has no dialog dependency.

- [ ] **Step 3: Implement export coordination and UI**

In `App.tsx`:

- Import Lucide `Download`, `SessionExportDialog`, and `tauriSessionExportDialog`.
- Add optional `sessionExportDialog` to `AppProps` and default it to the Tauri adapter.
- Add `"export"` to `LoadingState` and a matching loading-dialog copy entry.
- Add `detail?: string | null` to `CompletionNotice` so export success can show a path without calling it a backup.
- Implement `handleExportSession(row)` to choose the destination first, treat `null` as cancellation, set loading only after confirmation, call `migrationApi.exportSession`, and set completion detail to the returned path.
- Pass `onExport` and `exporting` to `SessionItem`.
- Render the last action button with `Download` and label `导出` or `导出中`.
- Add localized messages for every `session_export_*` error code from Tasks 1 and 2.

The handler must use the scanned `row.path` and current trimmed Codex Home:

```typescript
async function handleExportSession(row: ThreadRow) {
  try {
    const destinationPath = await sessionExportDialog.chooseDestination(row.path);
    if (!destinationPath) return;
    setLoading("export");
    setError(null);
    setCompletionNotice(null);
    const result = await migrationApi.exportSession({
      codexHome: codexHome.trim(),
      threadId: row.threadId,
      sourcePath: row.path,
      destinationPath
    });
    setCompletionNotice({
      message: `已导出会话：${row.displayName}`,
      detail: result.destinationPath
    });
  } catch (caught) {
    setError(errorMessage(caught));
  } finally {
    setLoading("idle");
  }
}
```

- [ ] **Step 4: Run App tests and verify GREEN**

```powershell
npm --workspace apps/desktop run test -- src/App.test.tsx
```

Expected: all App tests pass.

- [ ] **Step 5: Run the frontend test suite**

```powershell
npm test
```

Expected: all Vitest tests pass without warnings.

- [ ] **Step 6: Commit the user workflow**

```powershell
git add apps/desktop/src/App.tsx apps/desktop/src/App.test.tsx
git commit -m "feat: export sessions from the session list"
```

### Task 5: Documentation, Memory, And Full Verification

**Files:**
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `README.en.md`
- Modify: `MEMORY.md`

- [ ] **Step 1: Document the user-facing feature**

Add one feature bullet to each README stating that active and archived sessions can be exported as untouched raw JSONL through the native Save As dialog. Do not describe Markdown, bulk export, ZIP, or import.

- [ ] **Step 2: Record the durable project rule**

Append one dated `MEMORY.md` note:

```markdown
- 2026-07-14: 单会话导出固定为通过系统“另存为”逐字节复制原始 `.jsonl`；不复用会截断/重组内容的“查看记录”接口。Rust 后端必须校验源文件位于当前 Codex Home 的 `sessions` 或 `archived_sessions`，导出不修改源会话、索引或 SQLite，也不要求退出 Codex。
```

- [ ] **Step 3: Run formatting and full verification**

```powershell
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
npm test
npm run web:build
git diff --check
```

Expected: every command exits 0, all tests pass, and `git diff --check` prints nothing.

- [ ] **Step 4: Commit documentation and memory**

```powershell
git add README.md README.zh-CN.md README.en.md MEMORY.md
git commit -m "docs: document raw session export"
```

- [ ] **Step 5: Inspect final repository state**

```powershell
git status --short
git log -6 --oneline
```

Expected: working tree is clean and the export implementation commits follow the design/plan commits.
