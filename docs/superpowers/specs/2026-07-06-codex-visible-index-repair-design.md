# AI Session Migrator Codex 可见索引修复设计

Date: 2026-07-06

## Goal

给 AI Session Migrator 增加一个“修复 Codex 可见索引”能力，用来处理真实 Codex 会话文件仍然存在，但 Codex Desktop 左侧项目列表显示不完整的问题。

这个能力的核心目标不是迁移 provider，也不是移动会话文件，而是把 Codex Desktop 的本地可见 catalog 与真实会话源重新对齐。用户最终看到的效果应该是：migrator 扫描出的有效会话，可以重新出现在 Codex Desktop 的项目会话列表里。

## Current Evidence

本机排查显示 Codex 会话可见性至少有三层：

- `~/.codex/sessions/**/*.jsonl` 和 `~/.codex/archived_sessions/*.jsonl`：真实会话文件，是最可信的会话本体。
- `~/.codex/state_5.sqlite` 和 `~/.codex/sqlite/state_5.sqlite` 的 `threads`：Codex 旧/主线程状态索引，保存 title、cwd、provider、archive 状态、preview、时间等。
- `~/.codex/sqlite/codex-dev.db` 的 `local_thread_catalog`：Codex Desktop 左侧项目列表使用的本地 catalog。该表缺失时，真实会话仍在，但左栏可能不可见。

实际问题样本中，`sessions` 与根目录 `state_5.sqlite.threads` 都能看到多条 `fun-claw` 会话，但 `codex-dev.db.local_thread_catalog` 只同步到一条，且 `local_thread_catalog_sync_state.initial_build_complete=0`。所以 ai-session-migrator 扫描结果比 Codex 左栏更接近真实状态。

## Product Scope

In scope:

- 扫描 active 和 archived JSONL 会话。
- 读取 `state_5.sqlite.threads`、`sqlite/state_5.sqlite.threads`、`sqlite/codex-dev.db.local_thread_catalog`。
- 识别“会话文件存在但 Codex Desktop catalog 缺失”的线程。
- 预览将写入 `local_thread_catalog` 的记录。
- 应用修复前强制检测 Codex 进程是否仍在运行。
- 应用修复前强制备份相关 sqlite 文件及 WAL/SHM 文件。
- 只补齐缺失或明显 stale 的 catalog 记录，不移动、不删除、不改写 JSONL。
- 修复后提示用户重启 Codex Desktop 或重新扫描。

Out of scope:

- 删除、归档或迁移会话本体。
- 清空重建整个 `local_thread_catalog`。
- 修改 Codex 登录、provider、插件或云端状态。
- 依赖 Codex 私有进程通信接口。
- 修复完全损坏且无法打开的 sqlite 数据库。

## Approaches

### A. 补齐缺失 catalog 记录（推荐）

以 JSONL 会话文件为事实源，以 `state_5.sqlite.threads` 为可见标题和时间的优先补充源，只向 `local_thread_catalog` 插入缺失线程或更新明显不一致的基础字段。

为什么推荐：它是最小写入路径，不碰会话本体，不覆盖 Codex 现有 catalog 记录。对用户的影响是可见性恢复，同时把误伤范围控制在最小。

### B. 清空并重建 `local_thread_catalog`

先清空当前 host 的 catalog，再从所有 JSONL 和 state 记录重建。

不推荐作为默认能力。它可能更彻底，但会覆盖 Codex 自己未来写入的观察序列、远端/本地混合 host 状态或其他内部字段。只能作为后续高级恢复模式，且必须二次确认。

### C. 只重写 `session_index.jsonl`

补回 legacy index，不写 `codex-dev.db`。

不推荐。当前证据显示 Codex Desktop 左栏主要受 `local_thread_catalog` 影响，只修 `session_index.jsonl` 不能解决用户看到的“侧栏对不上”。

## Chosen Design

实现方案 A：新增独立的可见索引修复 workflow。

UI 上它应与 provider migration 分开，不把“迁移 provider”和“修复 Codex 可见索引”混成一个主操作。扫描结果中可以继续展示已有 issue codes，但新增 catalog 专用状态：

- `missing_catalog_entry`：JSONL 存在，但 `local_thread_catalog` 缺失。
- `catalog_cwd_mismatch`：catalog 中 cwd 与 JSONL/session state 的 cwd 不一致。
- `catalog_title_stale`：catalog 标题为空或明显旧于 sqlite 可见标题。
- `catalog_missing_source_detail`：catalog 缺少可定位来源信息。
- `catalog_db_missing`：`sqlite/codex-dev.db` 不存在或没有 catalog 表。
- `codex_running`：修复 apply 阶段阻塞项，不作为 scan 的普通问题。

默认只选择 active session 的 `missing_catalog_entry` 修复项。archived session 可以显示，但不默认写入 active project catalog，避免把用户已经归档的会话重新暴露成活跃项目会话。

## Data Flow

1. Scan:
   - 读取 `sessions` 与 `archived_sessions` 下的 rollout JSONL。
   - 从 `session_meta.payload` 解析 `id`、`cwd`、`source`、`thread_source`、`model_provider`、`timestamp`、`cli_version`。
   - 读取 `state_5.sqlite.threads` 作为标题、preview、更新时间、归档状态的优先来源。
   - 读取 `sqlite/codex-dev.db.local_thread_catalog` 作为 Codex Desktop 当前可见 catalog。
   - 合并为 `CatalogRepairRow`。

2. Preview:
   - 用户选择待修复线程。
   - Rust 生成 `CatalogRepairPlan`，列出每条线程将插入或更新的字段。
   - Preview 只读，不创建备份，不写 sqlite。

3. Apply:
   - 检测 Codex Desktop / Codex CLI 相关进程是否仍在运行。
   - 如果 Codex 运行中，返回 `codex_process_running`，要求用户先退出 Codex。
   - 备份 `sqlite/codex-dev.db`、`sqlite/codex-dev.db-wal`、`sqlite/codex-dev.db-shm`、`state_5.sqlite`、`state_5.sqlite-wal`、`state_5.sqlite-shm`、`sqlite/state_5.sqlite*`、`session_index.jsonl`。
   - 打开 `codex-dev.db`，开启事务。
   - 确保 `local_thread_catalog_hosts` 有 `host_id='local'`。
   - 对缺失项执行 insert；对 stale 项执行受限 update。
   - 更新 `local_thread_catalog_sync_state.initial_build_complete` 不作为默认写入目标，除非表为空且本次完成了全量 catalog backfill。默认保守保留 Codex 自己的 sync 状态。

## Catalog Field Mapping

`local_thread_catalog` 写入字段：

- `host_id`: `local`
- `thread_id`: JSONL/session meta thread id
- `display_title`: 优先 `session_index.thread_name`，其次 `state_5.threads.title`，最后 JSONL 派生标题
- `source_created_at`: 优先 state `created_at_ms / 1000`，其次 JSONL timestamp
- `source_updated_at`: 优先 state `recency_at_ms / 1000`，其次 `updated_at_ms / 1000`，最后 JSONL 文件 mtime
- `cwd`: JSONL `session_meta.payload.cwd`
- `source_kind`: JSONL `session_meta.payload.source`，缺失时用 `vscode`
- `source_detail`: 保持空字符串，除非后续确认 Codex 对该字段有稳定语义
- `model_provider`: JSONL provider
- `git_branch`: 优先 state `git_branch`
- `observation_sequence`: 当前最大值 + 1 递增
- `missing_candidate`: `0`

标题优先级必须保留用户可见改名：`session_index.jsonl.thread_name` > `state_5.sqlite.threads.title` > JSONL 内容派生标题。

## Tauri Command Contract

### `scan_codex_catalog_repair`

Input:

- `codexHome: string`

Output:

- `rows: CatalogRepairRow[]`
- `summary: CatalogRepairSummary`

Behavior:

- 只读。
- 不要求 Codex 关闭。
- 如果 `codex-dev.db` 缺失，返回可展示诊断，但不直接失败整个 scan。

### `preview_codex_catalog_repair`

Input:

- `codexHome: string`
- `threadIds: string[]`

Output:

- `plannedChanges: CatalogRepairChange[]`
- `dryRun: true`

Behavior:

- 只读。
- 校验选中线程仍能从 JSONL 找到。
- 返回字段级别 diff，供 UI 展示。

### `apply_codex_catalog_repair`

Input:

- `codexHome: string`
- `threadIds: string[]`

Output:

- `changedThreads: string[]`
- `backupDir: string`
- `dryRun: false`

Behavior:

- 写入前检查 Codex 进程。
- 写入前创建备份。
- 事务内完成 catalog insert/update。
- 不修改 JSONL。
- 不默认修改 `session_index.jsonl`。

## UI Design

新增一个独立操作区，文案使用“修复 Codex 可见索引”，避免用户误解为迁移 provider。

建议信息架构：

- Summary metrics:
  - 总会话
  - Codex 左栏可能不可见
  - 将修复
  - 已归档
- Row badges:
  - 项目
  - 活跃/已归档
  - catalog 缺失
  - 标题 stale
- Actions:
  - 预览修复
  - 确认修复

确认修复按钮在以下情况禁用：

- 未选择线程。
- preview 尚未生成。
- Codex 运行中。
- `codex-dev.db` 缺失且没有可创建表结构的明确策略。

Apply 失败时，UI 需要保留错误 code、错误说明和已创建的 backup path。

## Safety Model

- 默认不写 JSONL。
- 默认不改 provider。
- 默认不归档/激活。
- 默认不清空 catalog。
- Apply 前必须备份相关 sqlite、WAL、SHM 和 legacy index。
- Apply 前必须检查 Codex 进程，Codex 运行中拒绝写入。
- sqlite 写入必须使用事务。
- 每条 change 必须可在 preview 中看到。
- 如果 sqlite schema 与预期不同，返回 `catalog_schema_unsupported`，不要猜字段。

## Process Detection

Windows 默认检查进程名：

- `Codex`
- `Codex Desktop`
- `codex`

实现上不依赖 shell 文本解析，Rust 侧优先使用系统进程枚举库或 Windows API。测试中通过可注入的 process provider 模拟 Codex running / not running。

## Error Handling

Required error codes:

- `codex_home_missing`
- `sessions_missing`
- `no_sessions_found`
- `catalog_db_missing`
- `catalog_schema_unsupported`
- `no_threads_selected`
- `selected_thread_missing`
- `codex_process_running`
- `backup_create_failed`
- `sqlite_open_failed`
- `sqlite_busy_timeout_failed`
- `sqlite_transaction_failed`
- `catalog_insert_failed`
- `catalog_update_failed`

## Testing Plan

Rust tests:

- Scan detects missing catalog entries when JSONL exists but `local_thread_catalog` lacks the thread.
- Scan treats active and archived sessions differently.
- Scan preserves user-renamed title priority.
- Preview returns insert changes without writing.
- Apply refuses when process detector says Codex is running.
- Apply creates backup before writing.
- Apply inserts missing catalog rows in a transaction.
- Apply does not change JSONL bytes.
- Apply keeps existing catalog rows unless selected stale fields need restricted update.
- Schema mismatch returns `catalog_schema_unsupported`.

Frontend tests:

- Scan renders catalog repair metrics.
- Missing catalog rows are selected by default only for active sessions.
- Preview displays field-level changes.
- Confirm repair is blocked when Codex running state is reported.
- Success panel shows changed thread IDs and backup directory.
- Error panel shows actionable message for `codex_process_running`.

Manual verification:

- Build app.
- Use a copied `.codex` fixture with intentionally deleted `local_thread_catalog` rows.
- Scan shows missing catalog entries.
- Preview shows intended inserts.
- Close Codex Desktop.
- Apply repair.
- Restart Codex Desktop and confirm project sidebar shows restored conversations.

## Acceptance Criteria

- User can diagnose why ai-session-migrator sees more sessions than Codex Desktop.
- User can safely repair missing Codex Desktop catalog entries.
- Repair never modifies session JSONL content.
- Repair refuses to write while Codex is running.
- Repair creates restorable backups before sqlite writes.
- User-visible titles remain consistent with prior Codex/session_index renames.
- Existing provider migration workflow remains unaffected.
