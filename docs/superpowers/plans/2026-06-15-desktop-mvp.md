# Desktop MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Tauri + React desktop MVP for AI Session Migrator, focused on selecting local Codex sessions and migrating them from one provider to another.

**Architecture:** The repository starts as a Tauri desktop app with a React frontend and a Rust command layer. The first pass uses a typed mock data adapter in the frontend so the desktop workflow can be designed and verified before porting the Python migration core into Rust.

**Tech Stack:** Tauri 2, React, TypeScript, Vite, Rust, Vitest, Testing Library, Playwright-style browser verification.

---

## File Structure

- `README.md`: open-source product positioning, development commands, safety model.
- `.gitignore`: ignore Node, Rust, Tauri, build, editor, and OS artifacts.
- `package.json`: root scripts for desktop app install, dev, build, and tests.
- `apps/desktop/package.json`: frontend/Tauri app package metadata and scripts.
- `apps/desktop/index.html`: Vite HTML entry.
- `apps/desktop/src/main.tsx`: React root entry.
- `apps/desktop/src/App.tsx`: desktop migration assistant screen.
- `apps/desktop/src/App.test.tsx`: UI behavior tests.
- `apps/desktop/src/domain/session.ts`: shared TypeScript types and mock migration data.
- `apps/desktop/src/styles.css`: desktop UI styling.
- `apps/desktop/src-tauri/Cargo.toml`: Rust Tauri package manifest.
- `apps/desktop/src-tauri/tauri.conf.json`: Tauri app configuration.
- `apps/desktop/src-tauri/src/main.rs`: Rust Tauri app bootstrap and placeholder commands.

## Task 1: Repository Metadata

**Files:**
- Create: `README.md`
- Create: `.gitignore`
- Create: `package.json`

- [ ] **Step 1: Create README**

Create `README.md`:

```markdown
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
```

- [ ] **Step 2: Create .gitignore**

Create `.gitignore`:

```gitignore
node_modules/
dist/
target/
.vite/
.turbo/
*.log
*.tmp
.DS_Store
Thumbs.db
```

- [ ] **Step 3: Create root package.json**

Create `package.json`:

```json
{
  "name": "ai-session-migrator",
  "private": true,
  "version": "0.1.0",
  "description": "Local-first desktop tool for migrating AI assistant sessions between providers.",
  "scripts": {
    "dev": "npm --workspace apps/desktop run dev",
    "build": "npm --workspace apps/desktop run build",
    "test": "npm --workspace apps/desktop run test"
  },
  "workspaces": [
    "apps/desktop"
  ],
  "license": "MIT"
}
```

- [ ] **Step 4: Verify metadata exists**

Run:

```powershell
Get-ChildItem -Force
```

Expected: `README.md`, `.gitignore`, `package.json`, and `.git` are present.

## Task 2: Desktop App Scaffold

**Files:**
- Create: `apps/desktop/package.json`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/App.tsx`
- Create: `apps/desktop/src/domain/session.ts`
- Create: `apps/desktop/src/styles.css`

- [ ] **Step 1: Create desktop package.json**

Create `apps/desktop/package.json`:

```json
{
  "name": "@ai-session-migrator/desktop",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite --host 127.0.0.1",
    "build": "tsc && vite build",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "lucide-react": "^0.468.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.4.8",
    "@testing-library/react": "^16.0.1",
    "@testing-library/user-event": "^14.5.2",
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "typescript": "^5.5.4",
    "vite": "^5.4.0",
    "vitest": "^2.0.5"
  }
}
```

- [ ] **Step 2: Create Vite entry**

Create `apps/desktop/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>AI Session Migrator</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 3: Create domain types and mock data**

Create `apps/desktop/src/domain/session.ts`:

```ts
export type SessionStatus = "ready" | "needs_attention";

export type SessionRow = {
  id: string;
  shortId: string;
  title: string;
  sourceProvider: string;
  targetProvider: string;
  status: SessionStatus;
  reason: string;
  updatedAt: string;
};

export const mockSessions: SessionRow[] = [
  {
    id: "019cfc5a-acce-7b21-9f2d-5e1466fdb82f",
    shortId: "019cfc5a",
    title: "恢复 provider 切换后的会话",
    sourceProvider: "gmn",
    targetProvider: "yihubangg",
    status: "ready",
    reason: "这个会话属于旧 provider，可以迁移到当前 provider。",
    updatedAt: "2026-06-15 19:12"
  },
  {
    id: "019ecaae-e166-71f2-b520-43a23155bd3d",
    shortId: "019ecaae",
    title: "讨论会话迁移工具开源方向",
    sourceProvider: "funai",
    targetProvider: "yihubangg",
    status: "ready",
    reason: "这个会话可迁移，迁移前会自动创建备份。",
    updatedAt: "2026-06-15 18:43"
  }
];
```

- [ ] **Step 4: Create React UI**

Create `apps/desktop/src/App.tsx`:

```tsx
import { CheckCircle2, FolderOpen, RefreshCw, ShieldCheck } from "lucide-react";
import { useMemo, useState } from "react";
import { mockSessions } from "./domain/session";
import "./styles.css";

export default function App() {
  const [sourceProvider, setSourceProvider] = useState("");
  const [targetProvider, setTargetProvider] = useState("yihubangg");
  const [selectedIds, setSelectedIds] = useState<string[]>(mockSessions.map((session) => session.id));

  const visibleSessions = useMemo(() => {
    return mockSessions.filter((session) => !sourceProvider || session.sourceProvider === sourceProvider);
  }, [sourceProvider]);

  const selectedCount = selectedIds.length;

  function toggleSession(id: string) {
    setSelectedIds((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id]
    );
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div>
          <p className="eyebrow">AI Session Migrator</p>
          <h1>会话迁移助手</h1>
          <p className="muted">把本地会话从旧 provider 迁移到当前 provider。</p>
        </div>

        <label>
          Codex 目录
          <div className="path-input">
            <FolderOpen size={16} />
            <input value="C:\\Users\\jianrui\\.codex" readOnly />
          </div>
        </label>

        <label>
          从哪个 provider 迁出
          <input
            placeholder="留空自动识别"
            value={sourceProvider}
            onChange={(event) => setSourceProvider(event.target.value)}
          />
        </label>

        <label>
          迁移到哪个 provider
          <input value={targetProvider} onChange={(event) => setTargetProvider(event.target.value)} />
        </label>

        <button className="primary-button">
          <RefreshCw size={16} />
          扫描可迁移会话
        </button>
      </aside>

      <section className="workspace">
        <header className="summary">
          <div>
            <p className="eyebrow">Preview first</p>
            <h2>准备迁移 {selectedCount} 个会话</h2>
            <p className="muted">默认只预览。确认迁移前会创建备份，不会上传任何数据。</p>
          </div>
          <div className="summary-actions">
            <button className="secondary-button">预览迁移</button>
            <button className="primary-button">
              <ShieldCheck size={16} />
              确认迁移
            </button>
          </div>
        </header>

        <section className="steps" aria-label="Migration safety summary">
          <div>
            <CheckCircle2 size={18} />
            <span>更新 provider 标记</span>
          </div>
          <div>
            <CheckCircle2 size={18} />
            <span>补齐会话列表记录</span>
          </div>
          <div>
            <CheckCircle2 size={18} />
            <span>写入前自动备份</span>
          </div>
        </section>

        <section className="session-panel">
          <div className="panel-heading">
            <div>
              <h3>选择要迁移的会话</h3>
              <p className="muted">优先显示标题和时间，高级信息稍后再展开。</p>
            </div>
            <span className="pill">{visibleSessions.length} 个可见</span>
          </div>

          <div className="session-list">
            {visibleSessions.map((session) => (
              <label className="session-row" key={session.id}>
                <input
                  type="checkbox"
                  checked={selectedIds.includes(session.id)}
                  onChange={() => toggleSession(session.id)}
                />
                <div className="session-main">
                  <strong>{session.title}</strong>
                  <span>{session.reason}</span>
                </div>
                <div className="session-meta">
                  <span>{session.sourceProvider} → {targetProvider}</span>
                  <small>{session.updatedAt}</small>
                </div>
              </label>
            ))}
          </div>
        </section>
      </section>
    </main>
  );
}
```

- [ ] **Step 5: Create React entry**

Create `apps/desktop/src/main.tsx`:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 6: Create styling**

Create `apps/desktop/src/styles.css` with desktop app layout, restrained neutral palette, accessible focus states, and no emoji icons.

Use the exact CSS from implementation when executing this plan; keep the layout as two-pane desktop UI with a 320px sidebar and flexible workspace.

## Task 3: Frontend Tests

**Files:**
- Create: `apps/desktop/src/App.test.tsx`
- Create: `apps/desktop/vitest.config.ts`
- Create: `apps/desktop/src/test/setup.ts`

- [ ] **Step 1: Create Vitest config**

Create `apps/desktop/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"]
  }
});
```

- [ ] **Step 2: Create test setup**

Create `apps/desktop/src/test/setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 3: Create UI tests**

Create `apps/desktop/src/App.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";

test("renders the desktop migration workflow", () => {
  render(<App />);

  expect(screen.getByRole("heading", { name: "会话迁移助手" })).toBeInTheDocument();
  expect(screen.getByLabelText("迁移到哪个 provider")).toHaveValue("yihubangg");
  expect(screen.getByText("选择要迁移的会话")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /预览迁移/ })).toBeInTheDocument();
});

test("filters sessions by source provider", async () => {
  const user = userEvent.setup();
  render(<App />);

  await user.type(screen.getByLabelText("从哪个 provider 迁出"), "funai");

  expect(screen.getByText("讨论会话迁移工具开源方向")).toBeInTheDocument();
  expect(screen.queryByText("恢复 provider 切换后的会话")).not.toBeInTheDocument();
});
```

- [ ] **Step 4: Run tests**

Run:

```powershell
npm install
npm test
```

Expected: both tests pass.

## Task 4: Tauri Shell

**Files:**
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/src/main.rs`

- [ ] **Step 1: Create Cargo manifest**

Create `apps/desktop/src-tauri/Cargo.toml`:

```toml
[package]
name = "ai-session-migrator"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Create Tauri config**

Create `apps/desktop/src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "AI Session Migrator",
  "version": "0.1.0",
  "identifier": "dev.ai-session-migrator.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://127.0.0.1:5173",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "AI Session Migrator",
        "width": 1180,
        "height": 760,
        "minWidth": 960,
        "minHeight": 640
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": "all"
  }
}
```

- [ ] **Step 3: Create Rust main**

Create `apps/desktop/src-tauri/src/main.rs`:

```rust
#[tauri::command]
fn app_health() -> &'static str {
    "ok"
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_health])
        .run(tauri::generate_context!())
        .expect("failed to run AI Session Migrator");
}
```

- [ ] **Step 4: Verify Rust compiles**

Run:

```powershell
cd apps/desktop/src-tauri
cargo check
```

Expected: Rust package checks successfully.

## Task 5: Browser Verification

**Files:**
- Modify only if verification finds layout defects.

- [ ] **Step 1: Start frontend**

Run:

```powershell
npm run dev
```

Expected: Vite serves the desktop UI on `http://127.0.0.1:5173`.

- [ ] **Step 2: Verify in browser**

Open `http://127.0.0.1:5173` in the in-app Browser.

Expected:

- Page shows `会话迁移助手`.
- Sidebar shows Codex path, source provider, target provider, and scan action.
- Main area shows preview buttons and session checklist.
- No horizontal overlap at 1280x720.
- No emoji icons.

- [ ] **Step 3: Verify final checks**

Run:

```powershell
npm test
npm run build
```

Expected: tests and build pass.

## Self-Review

- Spec coverage: The plan covers repository metadata, desktop app scaffold, migration-focused UI, frontend tests, Tauri shell, and browser verification.
- Placeholder scan: The only deferred implementation is the Rust migration core, intentionally out of this MVP scope. The desktop UI is fully specified.
- Type consistency: `SessionRow`, `mockSessions`, and `App.tsx` use consistent names.
