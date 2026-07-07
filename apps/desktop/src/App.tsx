import {
  AlertTriangle,
  Archive,
  ArchiveRestore,
  ArrowRight,
  ArrowRightLeft,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  FileText,
  FolderOpen,
  HardDrive,
  ListFilter,
  MessageSquareText,
  Power,
  RefreshCw,
  Search,
  ShieldCheck,
  Trash2,
  Wrench
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { fallbackCodexHome, resolveDesktopCodexHome } from "./domain/defaultCodexHome";
import type { DesktopActions } from "./domain/desktopActions";
import { tauriDesktopActions } from "./domain/desktopActions";
import type { MigrationApi } from "./domain/migrationApi";
import { tauriMigrationApi } from "./domain/migrationApi";
import type {
  CatalogRepairRequest,
  CatalogRepairResult,
  CatalogRepairRow,
  CatalogRepairScanResponse,
  CommandError,
  MigrationRequest,
  MigrationResult,
  ProviderRestartResult,
  ScanResponse,
  SessionTranscript,
  ThreadRow
} from "./domain/session";
import SplashScreen from "./SplashScreen";
import "./styles.css";

const ALL_SOURCES = "__all__";
const CUSTOM_TARGET = "__custom__";

type DefaultCodexHomeResolver = () => Promise<string>;

type AppProps = {
  migrationApi?: MigrationApi;
  desktopActions?: DesktopActions;
  resolveDefaultCodexHome?: DefaultCodexHomeResolver;
  showStartupSplash?: boolean;
  startupSplashDurationMs?: number;
};

type LoadingState =
  | "idle"
  | "scan"
  | "preview"
  | "apply"
  | "catalogPreview"
  | "catalogApply"
  | "delete"
  | "archive"
  | "activate"
  | "restart";

type WorkspaceTab = "migration" | "catalogRepair";

type PendingLifecycleAction = {
  action: "archive" | "activate";
  row: ThreadRow;
};

type DisplayError = {
  message: string;
  operation?: string | null;
  backupDir?: string | null;
};

type CompletionNotice = {
  message: string;
  backupDir?: string | null;
  restartTargetProvider?: string | null;
  restartResult?: ProviderRestartResult | null;
};

export default function App({
  migrationApi = tauriMigrationApi,
  desktopActions = tauriDesktopActions,
  resolveDefaultCodexHome = resolveDesktopCodexHome,
  showStartupSplash = true,
  startupSplashDurationMs
}: AppProps) {
  const userEditedCodexHome = useRef(false);
  const transcriptRequestSeq = useRef(0);
  const [startupSplashVisible, setStartupSplashVisible] = useState(showStartupSplash);
  const [codexHome, setCodexHome] = useState(fallbackCodexHome);
  const [scanResponse, setScanResponse] = useState<ScanResponse | null>(null);
  const [sourceProvider, setSourceProvider] = useState(ALL_SOURCES);
  const [targetChoice, setTargetChoice] = useState("");
  const [customTargetProvider, setCustomTargetProvider] = useState("");
  const [activeWorkspaceTab, setActiveWorkspaceTab] = useState<WorkspaceTab>("migration");
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [detailsRow, setDetailsRow] = useState<ThreadRow | null>(null);
  const [transcriptRow, setTranscriptRow] = useState<ThreadRow | null>(null);
  const [transcript, setTranscript] = useState<SessionTranscript | null>(null);
  const [transcriptLoading, setTranscriptLoading] = useState(false);
  const [transcriptError, setTranscriptError] = useState<DisplayError | null>(null);
  const [previewResult, setPreviewResult] = useState<MigrationResult | null>(null);
  const [previewRequestKey, setPreviewRequestKey] = useState("");
  const [catalogRepair, setCatalogRepair] = useState<CatalogRepairScanResponse | null>(null);
  const [catalogRepairSelectedIds, setCatalogRepairSelectedIds] = useState<string[]>([]);
  const [catalogRepairPreview, setCatalogRepairPreview] = useState<CatalogRepairResult | null>(null);
  const [catalogRepairPreviewRequestKey, setCatalogRepairPreviewRequestKey] = useState("");
  const [completionNotice, setCompletionNotice] = useState<CompletionNotice | null>(null);
  const [loading, setLoading] = useState<LoadingState>("idle");
  const [error, setError] = useState<DisplayError | null>(null);
  const [confirmingApply, setConfirmingApply] = useState(false);
  const [pendingDeleteArchivedThreadIds, setPendingDeleteArchivedThreadIds] = useState<string[] | null>(null);
  const [pendingLifecycleAction, setPendingLifecycleAction] = useState<PendingLifecycleAction | null>(null);
  const [confirmingRestart, setConfirmingRestart] = useState(false);

  useEffect(() => {
    document.getElementById("preload-splash")?.remove();
  }, []);

  const rows = scanResponse?.dashboard.rows ?? [];
  const sourceProviderChoices = useMemo(
    () => (scanResponse ? sourceProviderOptions(scanResponse, sourceProvider) : []),
    [scanResponse, sourceProvider]
  );
  const visibleRows = useMemo(() => {
    const filteredRows =
      sourceProvider === ALL_SOURCES ? rows : rows.filter((row) => row.fileProvider === sourceProvider);
    return [...filteredRows].sort(compareSessionRows);
  }, [rows, sourceProvider]);
  const visibleThreadIds = useMemo(() => visibleRows.map((row) => row.threadId), [visibleRows]);
  const resolvedTargetProvider =
    targetChoice === CUSTOM_TARGET ? customTargetProvider.trim() : targetChoice.trim();
  const migrationRows = useMemo(
    () =>
      visibleRows.filter(
        (row) =>
          selectedIds.includes(row.threadId) &&
          row.canMigrate &&
          row.fileProvider !== resolvedTargetProvider
      ),
    [resolvedTargetProvider, selectedIds, visibleRows]
  );
  const migrationThreadIds = migrationRows.map((row) => row.threadId);
  const selectedArchivedRows = useMemo(
    () => visibleRows.filter((row) => selectedIds.includes(row.threadId) && row.lifecycle === "archived"),
    [selectedIds, visibleRows]
  );
  const deleteArchivedThreadIds = selectedArchivedRows.map((row) => row.threadId);
  const selectedVisibleCount = visibleRows.filter((row) => selectedIds.includes(row.threadId)).length;
  const currentMigrationRequest = useMemo<MigrationRequest | null>(() => {
    if (migrationThreadIds.length === 0 || resolvedTargetProvider.length === 0) {
      return null;
    }
    return {
      codexHome: codexHome.trim(),
      sourceProvider: sourceProvider === ALL_SOURCES ? null : sourceProvider,
      targetProvider: resolvedTargetProvider,
      threadIds: migrationThreadIds
    };
  }, [codexHome, migrationThreadIds, resolvedTargetProvider, sourceProvider]);
  const currentMigrationRequestKey = currentMigrationRequest ? requestKey(currentMigrationRequest) : "";
  const canPreview = currentMigrationRequest !== null && loading === "idle";
  const canApply = canPreview && previewResult !== null && previewRequestKey === currentMigrationRequestKey;
  const canDeleteArchived = scanResponse !== null && deleteArchivedThreadIds.length > 0 && loading === "idle";
  const catalogRepairableThreadIds = useMemo(
    () => new Set((catalogRepair?.rows ?? []).filter(hasRepairableCatalogIssue).map((row) => row.threadId)),
    [catalogRepair]
  );
  const catalogRepairThreadIds = useMemo(
    () => catalogRepairSelectedIds.filter((threadId) => catalogRepairableThreadIds.has(threadId)),
    [catalogRepairSelectedIds, catalogRepairableThreadIds]
  );
  const currentCatalogRepairRequest = useMemo<CatalogRepairRequest | null>(() => {
    if (catalogRepairThreadIds.length === 0) {
      return null;
    }
    return {
      codexHome: codexHome.trim(),
      threadIds: catalogRepairThreadIds
    };
  }, [catalogRepairThreadIds, codexHome]);
  const currentCatalogRepairRequestKey = currentCatalogRepairRequest ? requestKey(currentCatalogRepairRequest) : "";
  const canPreviewCatalogRepair = currentCatalogRepairRequest !== null && loading === "idle";
  const canApplyCatalogRepair =
    canPreviewCatalogRepair &&
    catalogRepairPreview !== null &&
    catalogRepairPreviewRequestKey === currentCatalogRepairRequestKey;
  const effectiveWorkspaceTab: WorkspaceTab = scanResponse ? activeWorkspaceTab : "migration";
  const isCatalogRepairTab = effectiveWorkspaceTab === "catalogRepair";
  const catalogRepairIssueCount = useMemo(
    () => (catalogRepair?.rows ?? []).filter(hasRepairableCatalogIssue).length,
    [catalogRepair]
  );
  const workspaceTitle = isCatalogRepairTab
    ? "会话修复"
    : summaryTitle(scanResponse, migrationThreadIds.length);
  const workspaceSubtitle = isCatalogRepairTab
    ? "Repair Codex Session Visibility"
    : englishSummaryTitle(scanResponse, migrationThreadIds.length);
  const workspaceMutedText = isCatalogRepairTab
    ? catalogRepair
      ? `已发现 ${catalogRepairIssueCount} 条可修复会话可见性问题，默认选择 ${catalogRepair.summary.selectedByDefault} 条活跃会话。`
      : "扫描后查看 Codex Desktop 会话可见性状态。"
    : scanResponse
      ? `已发现 ${scanResponse.dashboard.totalThreads} 个会话，其中 ${scanResponse.dashboard.problemThreads} 个需要处理。`
      : "先扫描本地 Codex 目录，确认后再迁移。";

  useEffect(() => {
    let cancelled = false;

    resolveDefaultCodexHome()
      .then((resolvedHome) => {
        if (!cancelled && resolvedHome.trim().length > 0 && !userEditedCodexHome.current) {
          setCodexHome(resolvedHome);
        }
      })
      .catch(() => {
        // Browser dev mode may not expose Tauri path APIs; the editable fallback remains available.
      });

    return () => {
      cancelled = true;
    };
  }, [resolveDefaultCodexHome]);

  async function handleScan() {
    if (codexHome.trim().length === 0) {
      clearMigrationContext();
      setError({ message: "请输入 Codex 目录" });
      return;
    }

    setLoading("scan");
    setError(null);
    clearMigrationContext();
    try {
      const response = await migrationApi.scanCodexHome(codexHome.trim());
      const catalogResponse = await migrationApi.scanCodexCatalogRepair(codexHome.trim());
      const defaultSourceProvider = defaultSourceForScan(response);
      setScanResponse(response);
      setCatalogRepair(catalogResponse);
      setCatalogRepairSelectedIds(defaultCatalogRepairThreadIds(catalogResponse));
      setSourceProvider(defaultSourceProvider);
      const firstTarget =
        response.providerOptions.currentConfigProvider ??
        response.providerOptions.targetProviders[0]?.value ??
        "";
      setTargetChoice(firstTarget);
      setCustomTargetProvider("");
      setSelectedIds(threadIdsForSource(response.dashboard.rows, defaultSourceProvider));
      setDetailsRow(null);
      closeTranscript();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading("idle");
    }
  }

  async function handleCatalogRepairPreview() {
    if (!currentCatalogRepairRequest) {
      return;
    }
    const request = currentCatalogRepairRequest;
    const requestKeyForRun = currentCatalogRepairRequestKey;
    setLoading("catalogPreview");
    setError(null);
    setCatalogRepairPreview(null);
    setCatalogRepairPreviewRequestKey("");
    setCompletionNotice(null);
    try {
      const result = await migrationApi.previewCodexCatalogRepair(request);
      setCatalogRepairPreview(result);
      setCatalogRepairPreviewRequestKey(requestKeyForRun);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading("idle");
    }
  }

  async function handleCatalogRepairApply() {
    if (!currentCatalogRepairRequest || !canApplyCatalogRepair) {
      return;
    }
    const request = currentCatalogRepairRequest;
    setLoading("catalogApply");
    setError(null);
    setCompletionNotice(null);
    try {
      const result = await migrationApi.applyCodexCatalogRepair(request);
      setCatalogRepairPreview(null);
      setCatalogRepairPreviewRequestKey("");
      setCompletionNotice({
        message: `已修复 ${result.changedThreads.length} 条 Codex 会话可见性记录`,
        backupDir: result.backupDir
      });
      await refreshCatalogRepairScan();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading("idle");
    }
  }

  async function handlePreview() {
    await runMigration("preview");
  }

  function handleApply() {
    if (canApply) {
      setConfirmingApply(true);
    }
  }

  async function confirmApply() {
    setConfirmingApply(false);
    await runMigration("apply");
  }

  async function runMigration(mode: "preview" | "apply") {
    if (!currentMigrationRequest || (mode === "apply" && !canApply)) {
      return;
    }
    const request = currentMigrationRequest;
    const requestKeyForRun = currentMigrationRequestKey;
    setLoading(mode);
    setError(null);
    if (mode === "preview") {
      setPreviewResult(null);
      setPreviewRequestKey("");
    } else {
      setPreviewResult(null);
    }
    setCompletionNotice(null);
    try {
      const result =
        mode === "preview"
          ? await migrationApi.previewProviderMigration(request)
          : await migrationApi.applyProviderMigration(request);
      if (mode === "preview") {
        setPreviewResult(result);
        setPreviewRequestKey(requestKeyForRun);
      } else {
        await refreshScanAfterMigration(request.sourceProvider, result.changedThreads);
        setCompletionNotice({
          message: `已完成迁移 ${result.changedThreads.length} 个会话`,
          backupDir: result.backupDir,
          restartTargetProvider: request.targetProvider,
          restartResult: null
        });
        setConfirmingRestart(true);
      }
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading("idle");
    }
  }

  function handleDeleteArchived() {
    if (canDeleteArchived) {
      clearRunResults();
      setPendingDeleteArchivedThreadIds(deleteArchivedThreadIds);
    }
  }

  function handleDeleteArchivedRow(row: ThreadRow) {
    if (row.lifecycle === "archived" && loading === "idle") {
      clearRunResults();
      setPendingDeleteArchivedThreadIds([row.threadId]);
    }
  }

  function handleArchiveRow(row: ThreadRow) {
    if (row.lifecycle === "active" && loading === "idle") {
      clearRunResults();
      setPendingLifecycleAction({ action: "archive", row });
    }
  }

  function handleActivateRow(row: ThreadRow) {
    if (row.lifecycle === "archived" && loading === "idle") {
      clearRunResults();
      setPendingLifecycleAction({ action: "activate", row });
    }
  }

  function handleRestartCodex() {
    if (completionNotice?.restartTargetProvider && loading === "idle") {
      setConfirmingRestart(true);
    }
  }

  async function confirmRestartCodex() {
    const targetProvider = completionNotice?.restartTargetProvider;
    if (!targetProvider) {
      setConfirmingRestart(false);
      return;
    }
    setConfirmingRestart(false);
    setLoading("restart");
    setError(null);
    try {
      const result = await migrationApi.switchProviderAndRestart({
        codexHome: codexHome.trim(),
        targetProvider
      });
      setCompletionNotice((current) =>
        current
          ? {
              ...current,
              message: result.restartMessage,
              backupDir: result.configBackupDir ?? current.backupDir,
              restartTargetProvider: null,
              restartResult: result
            }
          : current
      );
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading("idle");
    }
  }

  async function confirmDeleteArchived() {
    const threadIds = pendingDeleteArchivedThreadIds ?? [];
    if (!scanResponse || threadIds.length === 0 || loading !== "idle") {
      setPendingDeleteArchivedThreadIds(null);
      return;
    }
    const request = {
      codexHome: codexHome.trim(),
      threadIds
    };
    setPendingDeleteArchivedThreadIds(null);
    setLoading("delete");
    setError(null);
    setPreviewResult(null);
    setPreviewRequestKey("");
    setCompletionNotice(null);
    try {
      const result = await migrationApi.applyDeleteArchivedSessions(request);
      removeDeletedRows(result.deletedThreads);
      setCompletionNotice({
        message: `已删除 ${result.deletedThreads.length} 个归档会话`,
        backupDir: result.backupDir
      });
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading("idle");
    }
  }

  async function confirmLifecycleAction() {
    if (!pendingLifecycleAction || loading !== "idle") {
      setPendingLifecycleAction(null);
      return;
    }
    const pending = pendingLifecycleAction;
    setPendingLifecycleAction(null);
    setLoading(pending.action);
    setError(null);
    setPreviewResult(null);
    setPreviewRequestKey("");
    setCompletionNotice(null);
    try {
      const request = {
        codexHome: codexHome.trim(),
        threadIds: [pending.row.threadId]
      };
      const result =
        pending.action === "archive"
          ? await migrationApi.applyArchiveSessions(request)
          : await migrationApi.applyActivateSessions(request);
      await refreshScanAfterLifecycleChange(result.changedThreads);
      setCompletionNotice({
        message:
          pending.action === "archive"
            ? `已归档 ${result.changedThreads.length} 个会话`
            : `已激活 ${result.changedThreads.length} 个会话`,
        backupDir: result.backupDir
      });
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setLoading("idle");
    }
  }

  function removeDeletedRows(deletedThreads: string[]) {
    const deleted = new Set(deletedThreads);
    setScanResponse((current) => {
      if (!current) {
        return current;
      }
      const remainingRows = current.dashboard.rows.filter((row) => !deleted.has(row.threadId));
      return {
        ...current,
        dashboard: {
          ...current.dashboard,
          totalThreads: remainingRows.length,
          problemThreads: remainingRows.filter((row) => row.issueCodes.length > 0).length,
          issueCounts: countIssues(remainingRows),
          rows: remainingRows
        }
      };
    });
    setSelectedIds((current) => current.filter((threadId) => !deleted.has(threadId)));
    setDetailsRow((current) => (current && deleted.has(current.threadId) ? null : current));
    if (transcriptRow && deleted.has(transcriptRow.threadId)) {
      closeTranscript();
    }
  }

  function toggleSession(threadId: string) {
    clearRunResults();
    setSelectedIds((current) =>
      current.includes(threadId) ? current.filter((item) => item !== threadId) : [...current, threadId]
    );
  }

  function selectVisibleSessions() {
    clearRunResults();
    setSelectedIds((current) => Array.from(new Set([...current, ...visibleThreadIds])));
  }

  function clearVisibleSessions() {
    const visible = new Set(visibleThreadIds);
    clearRunResults();
    setSelectedIds((current) => current.filter((threadId) => !visible.has(threadId)));
  }

  function openDetails(row: ThreadRow) {
    setDetailsRow(row);
  }

  async function openTranscript(row: ThreadRow) {
    if (loading !== "idle") {
      return;
    }
    setTranscriptRow(row);
    setTranscript(null);
    setTranscriptError(null);
    setTranscriptLoading(true);
    const requestSeq = ++transcriptRequestSeq.current;
    try {
      const loadedTranscript = await migrationApi.readSessionTranscript({
        codexHome: codexHome.trim(),
        threadId: row.threadId,
        path: row.path
      });
      if (requestSeq !== transcriptRequestSeq.current) {
        return;
      }
      setTranscript(loadedTranscript);
    } catch (caught) {
      if (requestSeq !== transcriptRequestSeq.current) {
        return;
      }
      setTranscriptError(errorMessage(caught));
    } finally {
      if (requestSeq === transcriptRequestSeq.current) {
        setTranscriptLoading(false);
      }
    }
  }

  function closeTranscript() {
    transcriptRequestSeq.current += 1;
    setTranscriptRow(null);
    setTranscript(null);
    setTranscriptError(null);
    setTranscriptLoading(false);
  }

  function handleCodexHomeChange(value: string) {
    userEditedCodexHome.current = true;
    setCodexHome(value);
    clearMigrationContext();
    setError(null);
  }

  function clearMigrationContext() {
    setConfirmingApply(false);
    setPendingDeleteArchivedThreadIds(null);
    setPendingLifecycleAction(null);
    setConfirmingRestart(false);
    setScanResponse(null);
    setCatalogRepair(null);
    setCatalogRepairSelectedIds([]);
    setCatalogRepairPreview(null);
    setCatalogRepairPreviewRequestKey("");
    setSourceProvider(ALL_SOURCES);
    setTargetChoice("");
    setCustomTargetProvider("");
    setSelectedIds([]);
    setDetailsRow(null);
    closeTranscript();
    setPreviewResult(null);
    setPreviewRequestKey("");
    setCompletionNotice(null);
  }

  function clearRunResults() {
    setConfirmingApply(false);
    setPendingDeleteArchivedThreadIds(null);
    setPendingLifecycleAction(null);
    setConfirmingRestart(false);
    setPreviewResult(null);
    setPreviewRequestKey("");
    setCatalogRepairPreview(null);
    setCatalogRepairPreviewRequestKey("");
    setCompletionNotice(null);
  }

  function toggleCatalogRepairSession(threadId: string) {
    setCatalogRepairSelectedIds((current) =>
      current.includes(threadId) ? current.filter((item) => item !== threadId) : [...current, threadId]
    );
    setCatalogRepairPreview(null);
    setCatalogRepairPreviewRequestKey("");
    setCompletionNotice(null);
  }

  async function refreshScanAfterMigration(previousSourceProvider: string | null, changedThreads: string[]) {
    const refreshed = await migrationApi.scanCodexHome(codexHome.trim());
    const nextSourceProvider = previousSourceProvider ?? ALL_SOURCES;
    applyRefreshedScan(refreshed, nextSourceProvider, changedThreads);
  }

  async function refreshScanAfterLifecycleChange(changedThreads: string[]) {
    const refreshed = await migrationApi.scanCodexHome(codexHome.trim());
    applyRefreshedScan(refreshed, sourceProvider, changedThreads);
  }

  async function refreshCatalogRepairScan() {
    const refreshed = await migrationApi.scanCodexCatalogRepair(codexHome.trim());
    setCatalogRepair(refreshed);
    setCatalogRepairSelectedIds(defaultCatalogRepairThreadIds(refreshed));
  }

  function applyRefreshedScan(refreshed: ScanResponse, nextSourceProvider: string, changedThreads: string[]) {
    const existingThreadIds = new Set(refreshed.dashboard.rows.map((row) => row.threadId));
    const changedThreadIds = new Set(changedThreads);
    setScanResponse(refreshed);
    setSourceProvider(nextSourceProvider);
    setSelectedIds((current) =>
      current.filter((threadId) => existingThreadIds.has(threadId) && !changedThreadIds.has(threadId))
    );
    setDetailsRow((current) =>
      current && (!existingThreadIds.has(current.threadId) || changedThreadIds.has(current.threadId)) ? null : current
    );
    if (
      transcriptRow &&
      (!existingThreadIds.has(transcriptRow.threadId) || changedThreadIds.has(transcriptRow.threadId))
    ) {
      closeTranscript();
    }
  }

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="迁移设置">
        <div className="brand-block">
          <span className="brand-mark">
            <ArrowRightLeft aria-hidden="true" size={22} />
          </span>
          <div>
            <h1>AI 会话迁移</h1>
            <p className="eyebrow">AI Session Migrator</p>
          </div>
        </div>

        <section className="setup-card" aria-label="迁移步骤">
          <div className="step-row">
            <span className="step-number">1</span>
            <div>
              <strong>选择 Codex 数据目录</strong>
              <p>默认使用当前用户的 .codex，也可以手动改到其他目录。</p>
            </div>
          </div>

          <label className="field-label">
            Codex 目录
            <div className="path-input">
              <FolderOpen aria-hidden="true" size={17} />
              <input
                value={codexHome}
                onChange={(event) => {
                  handleCodexHomeChange(event.target.value);
                }}
              />
            </div>
          </label>

          <button className="primary-button" type="button" disabled={loading !== "idle"} onClick={handleScan}>
            {loading === "scan" ? <RefreshCw aria-hidden="true" size={17} /> : <Search aria-hidden="true" size={18} />}
            {loading === "scan" ? "正在扫描" : "扫描会话"}
          </button>

          <div className="step-row">
            <span className="step-number">2</span>
            <div>
              <strong>选择迁移方向</strong>
              <p>来源用于筛选会话，目标 provider 会写入选中的会话。</p>
            </div>
          </div>

          <div className="provider-route">
            <label className="provider-card field-label">
              来源 provider
              <select
                value={sourceProvider}
                onChange={(event) => {
                  const nextSourceProvider = event.target.value;
                  setSourceProvider(nextSourceProvider);
                  setSelectedIds(threadIdsForSource(rows, nextSourceProvider));
                  clearRunResults();
                }}
                disabled={!scanResponse}
              >
                <option value={ALL_SOURCES}>全部 provider ({rows.length})</option>
                {sourceProviderChoices.map((provider) => (
                  <option key={provider.value} value={provider.value}>
                    {provider.label}
                  </option>
                ))}
              </select>
            </label>

            <span className="route-arrow" aria-hidden="true">
              <ArrowRight size={18} />
            </span>

            <label className="provider-card provider-card-target field-label">
              目标 provider
              <select
                value={targetChoice}
                onChange={(event) => {
                  setTargetChoice(event.target.value);
                  clearRunResults();
                }}
                disabled={!scanResponse}
              >
                <option value="" disabled>
                  先扫描后选择
                </option>
                {scanResponse?.providerOptions.targetProviders.map((option) => (
                  <option key={`${option.kind}-${option.value}`} value={option.value}>
                    {option.label}
                  </option>
                ))}
                <option value={CUSTOM_TARGET}>自定义 provider...</option>
              </select>
            </label>
          </div>

          {targetChoice === CUSTOM_TARGET ? (
            <label className="field-label">
              自定义目标 provider
              <input
                value={customTargetProvider}
                onChange={(event) => {
                  setCustomTargetProvider(event.target.value);
                  clearRunResults();
                }}
                placeholder="例如 yihubangg"
              />
            </label>
          ) : null}
        </section>

        <section className="local-note" aria-label="本地安全说明">
          <HardDrive aria-hidden="true" size={18} />
          <div>
            <strong>本地处理</strong>
            <span>扫描、预览、备份、迁移和归档删除都在你的电脑上完成。</span>
          </div>
        </section>
      </aside>

      <section className="workspace">
        <header className="summary">
          <div>
            <h2>{workspaceTitle}</h2>
            <p className="summary-subtitle">{workspaceSubtitle}</p>
            <p className="muted">{workspaceMutedText}</p>
          </div>
          {isCatalogRepairTab ? null : (
            <div className="summary-actions">
              <button className="secondary-button" type="button" disabled={!canPreview} onClick={handlePreview}>
                {loading === "preview" ? "正在预览" : "预览迁移"}
              </button>
              <button className="primary-button" type="button" disabled={!canApply} onClick={handleApply}>
                <ShieldCheck aria-hidden="true" size={17} />
                {loading === "apply" ? "正在迁移" : "确认迁移"}
              </button>
            </div>
          )}
        </header>

        {error ? <ErrorPanel error={error} /> : null}
        {completionNotice ? (
          <CompletionNoticePanel
            notice={completionNotice}
            loading={loading}
            desktopActions={desktopActions}
            onRestartCodex={handleRestartCodex}
            onActionError={(message) => setError({ message })}
          />
        ) : null}

        {scanResponse ? (
          <WorkspaceTabs
            activeTab={effectiveWorkspaceTab}
            migrationCount={migrationThreadIds.length}
            repairCount={catalogRepairIssueCount}
            onChange={setActiveWorkspaceTab}
          />
        ) : null}

        {isCatalogRepairTab ? (
          <section
            id="workspace-panel-catalog-repair"
            className="workspace-tab-panel"
            role="tabpanel"
            aria-labelledby="workspace-tab-catalog-repair"
          >
            {catalogRepair ? (
              <CatalogRepairPanel
                response={catalogRepair}
                selectedIds={catalogRepairSelectedIds}
                selectedRepairableCount={catalogRepairThreadIds.length}
                preview={catalogRepairPreview}
                disabled={loading !== "idle"}
                canPreview={canPreviewCatalogRepair}
                canApply={canApplyCatalogRepair}
                onToggleSelected={toggleCatalogRepairSession}
                onPreview={handleCatalogRepairPreview}
                onApply={handleCatalogRepairApply}
              />
            ) : (
              <div className="empty-state">扫描后查看 Codex 会话修复项。</div>
            )}
          </section>
        ) : (
          <section
            id="workspace-panel-migration"
            className="workspace-tab-panel"
            role="tabpanel"
            aria-labelledby="workspace-tab-migration"
          >
            <section className="metrics" aria-label="扫描摘要">
              <Metric label="总会话" value={scanResponse?.dashboard.totalThreads ?? 0} />
              <Metric label="需处理" value={scanResponse?.dashboard.problemThreads ?? 0} />
              <Metric label="将迁移" value={migrationThreadIds.length} />
              <Metric label="可见会话" value={visibleRows.length} />
            </section>

            {previewResult ? (
              <ResultPanel title={`将更新 ${previewResult.changedThreads.length} 个会话`} result={previewResult} />
            ) : null}

            <section className="session-panel">
          <div className="panel-heading">
            <div className="bulk-actions" aria-label="批量选择会话">
              <button
                className="ghost-button"
                type="button"
                disabled={!scanResponse || visibleThreadIds.length === 0 || loading !== "idle"}
                onClick={selectVisibleSessions}
              >
                全选
              </button>
              <button
                className="ghost-button"
                type="button"
                disabled={!scanResponse || visibleThreadIds.length === 0 || loading !== "idle"}
                onClick={clearVisibleSessions}
              >
                取消全选
              </button>
              <button
                className="ghost-button danger-button"
                type="button"
                data-testid="delete-archived-button"
                disabled={!canDeleteArchived}
                onClick={handleDeleteArchived}
              >
                <Trash2 aria-hidden="true" size={15} />
                删除已选归档 {deleteArchivedThreadIds.length > 0 ? `(${deleteArchivedThreadIds.length})` : ""}
              </button>
            </div>
            <div className="visible-count">
              <ListFilter aria-hidden="true" size={15} />
              <span>正在显示 {visibleRows.length} 个项目</span>
              <span className="pill">
                {visibleRows.length} 个可见，{selectedVisibleCount} 个已选
              </span>
            </div>
          </div>

          {scanResponse ? (
            <div className="session-list">
              <div className="session-table-head" aria-hidden="true">
                <span />
                <span>会话详情</span>
                <span>时间</span>
                <span>操作</span>
              </div>
              {visibleRows.map((row) => (
                <SessionItem
                  key={row.threadId}
                  row={row}
                  selected={selectedIds.includes(row.threadId)}
                  disabled={loading !== "idle"}
                  onToggleSelected={() => toggleSession(row.threadId)}
                  onOpenDetails={() => openDetails(row)}
                  onOpenTranscript={() => openTranscript(row)}
                  onArchive={() => handleArchiveRow(row)}
                  onActivate={() => handleActivateRow(row)}
                  onDeleteArchived={() => handleDeleteArchivedRow(row)}
                />
              ))}
              {visibleRows.length === 0 ? <div className="empty-state">{emptyVisibleRowsText(sourceProvider)}</div> : null}
            </div>
          ) : (
            <div className="empty-state">还没有扫描结果。</div>
          )}
            </section>
          </section>
        )}
      </section>

      {confirmingApply ? (
        <ConfirmMigrationDialog
          migrationCount={migrationThreadIds.length}
          targetProvider={resolvedTargetProvider}
          onCancel={() => setConfirmingApply(false)}
          onConfirm={confirmApply}
        />
      ) : null}

      {loading !== "idle" ? <OperationLoadingDialog loading={loading} /> : null}

      {pendingDeleteArchivedThreadIds ? (
        <ConfirmDeleteArchivedDialog
          deleteCount={pendingDeleteArchivedThreadIds.length}
          onCancel={() => setPendingDeleteArchivedThreadIds(null)}
          onConfirm={confirmDeleteArchived}
        />
      ) : null}

      {pendingLifecycleAction ? (
        <ConfirmLifecycleDialog
          action={pendingLifecycleAction.action}
          row={pendingLifecycleAction.row}
          onCancel={() => setPendingLifecycleAction(null)}
          onConfirm={confirmLifecycleAction}
        />
      ) : null}

      {detailsRow ? (
        <SessionDetailsDialog
          row={detailsRow}
          targetProvider={resolvedTargetProvider || "未选择"}
          onClose={() => setDetailsRow(null)}
        />
      ) : null}

      {transcriptRow ? (
        <SessionTranscriptDialog
          row={transcriptRow}
          transcript={transcript}
          loading={transcriptLoading}
          error={transcriptError}
          onClose={closeTranscript}
        />
      ) : null}

      {confirmingRestart && completionNotice?.restartTargetProvider ? (
        <ConfirmRestartDialog
          targetProvider={completionNotice.restartTargetProvider}
          onCancel={() => setConfirmingRestart(false)}
          onConfirm={confirmRestartCodex}
        />
      ) : null}

      {startupSplashVisible ? (
        <SplashScreen durationMs={startupSplashDurationMs} onComplete={() => setStartupSplashVisible(false)} />
      ) : null}
    </main>
  );
}

function WorkspaceTabs({
  activeTab,
  migrationCount,
  repairCount,
  onChange
}: {
  activeTab: WorkspaceTab;
  migrationCount: number;
  repairCount: number;
  onChange: (tab: WorkspaceTab) => void;
}) {
  return (
    <nav className="workspace-tabs" role="tablist" aria-label="工作区视图">
      <button
        id="workspace-tab-migration"
        className={activeTab === "migration" ? "workspace-tab active" : "workspace-tab"}
        type="button"
        role="tab"
        aria-selected={activeTab === "migration"}
        aria-controls="workspace-panel-migration"
        onClick={() => onChange("migration")}
      >
        <span>会话迁移</span>
        <small>{migrationCount} 个将迁移</small>
      </button>
      <button
        id="workspace-tab-catalog-repair"
        className={activeTab === "catalogRepair" ? "workspace-tab active" : "workspace-tab"}
        type="button"
        role="tab"
        aria-selected={activeTab === "catalogRepair"}
        aria-controls="workspace-panel-catalog-repair"
        onClick={() => onChange("catalogRepair")}
      >
        <span>会话修复</span>
        {repairCount > 0 ? <strong className="workspace-tab-badge">{repairCount}</strong> : null}
      </button>
    </nav>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function SessionItem({
  row,
  selected,
  disabled,
  onToggleSelected,
  onOpenDetails,
  onOpenTranscript,
  onArchive,
  onActivate,
  onDeleteArchived
}: {
  row: ThreadRow;
  selected: boolean;
  disabled: boolean;
  onToggleSelected: () => void;
  onOpenDetails: () => void;
  onOpenTranscript: () => void;
  onArchive: () => void;
  onActivate: () => void;
  onDeleteArchived: () => void;
}) {
  const lifecycleClassName =
    row.lifecycle === "archived" ? "session-row session-row-archived" : "session-row session-row-active";

  return (
    <article className={lifecycleClassName} aria-label={row.displayName}>
      <label className="session-select">
        <input
          aria-label={`选择会话：${row.displayName}`}
          type="checkbox"
          checked={selected}
          onChange={onToggleSelected}
        />
        <span />
      </label>

      <div className="session-main">
        <span className="session-icon">
          <FileText aria-hidden="true" size={20} />
        </span>
        <div className="session-copy">
          <div className="session-title-wrap">
            <strong>{row.displayName}</strong>
            <span className={`lifecycle-badge ${row.lifecycle}`}>{lifecycleLabel(row.lifecycle)}</span>
            {row.projectName ? (
              <span className="project-badge" title={row.projectPath ?? row.projectName}>
                <FolderOpen aria-hidden="true" size={13} />
                项目：{row.projectName}
              </span>
            ) : null}
          </div>
          <p>{issueSummary(row.issueCodes)}</p>
        </div>
      </div>

      <div className="session-meta">
        <strong>{formatDate(row.updatedAtMs)}</strong>
        <small>{row.shortId}</small>
        <button className="ghost-button" type="button" onClick={onOpenDetails}>
          <ChevronRight aria-hidden="true" size={16} />
          高级信息
        </button>
      </div>

      <div className="session-actions" aria-label="会话操作">
        <button className="row-action-button" type="button" disabled={disabled} onClick={onOpenTranscript}>
          <MessageSquareText aria-hidden="true" size={15} />
          查看记录
        </button>
        {row.lifecycle === "active" ? (
          <button className="row-action-button" type="button" disabled={disabled} onClick={onArchive}>
            <Archive aria-hidden="true" size={15} />
            归档
          </button>
        ) : (
          <>
            <button className="row-action-button" type="button" disabled={disabled} onClick={onActivate}>
              <ArchiveRestore aria-hidden="true" size={15} />
              激活
            </button>
            <button className="row-action-button danger" type="button" disabled={disabled} onClick={onDeleteArchived}>
              <Trash2 aria-hidden="true" size={15} />
              删除
            </button>
          </>
        )}
      </div>
    </article>
  );
}

function CatalogRepairPanel({
  response,
  selectedIds,
  selectedRepairableCount,
  preview,
  disabled,
  canPreview,
  canApply,
  onToggleSelected,
  onPreview,
  onApply
}: {
  response: CatalogRepairScanResponse;
  selectedIds: string[];
  selectedRepairableCount: number;
  preview: CatalogRepairResult | null;
  disabled: boolean;
  canPreview: boolean;
  canApply: boolean;
  onToggleSelected: (threadId: string) => void;
  onPreview: () => void;
  onApply: () => void;
}) {
  const repairRows = response.rows.filter((row) => row.repairCodes.length > 0);
  const rolloutCompatibilityIssues = repairRows.filter((row) =>
    row.repairCodes.includes("rollout_internal_metadata_passthrough")
  ).length;

  return (
    <section className="catalog-repair-panel" aria-label="修复 Codex 会话可见性">
      <div className="catalog-repair-heading">
        <div>
          <p className="eyebrow">Codex Visibility</p>
          <h3>修复 Codex 会话可见性</h3>
          <p>
            补齐 Codex 左栏可见性元数据：缺失 catalog {response.summary.missingCatalogEntries} 条，缺失
            session_index {response.summary.missingSessionIndexEntries} 条，state 待修复{" "}
            {response.summary.stateMetadataIssues} 条，会话兼容性 {rolloutCompatibilityIssues} 条，默认选择{" "}
            {response.summary.selectedByDefault} 条活跃会话。
          </p>
        </div>
        <button className="secondary-button" type="button" disabled={!canPreview} onClick={onPreview}>
          <Wrench aria-hidden="true" size={17} />
          生成修复预览
        </button>
      </div>

      <div className="catalog-repair-stats" aria-label="Codex catalog 修复摘要">
        <div>
          <span>总会话</span>
          <strong>{response.summary.totalThreads}</strong>
        </div>
        <div>
          <span>缺失 catalog</span>
          <strong>{response.summary.missingCatalogEntries}</strong>
        </div>
        <div>
          <span>缺失 index</span>
          <strong>{response.summary.missingSessionIndexEntries}</strong>
        </div>
        <div>
          <span>state 待修复</span>
          <strong>{response.summary.stateMetadataIssues}</strong>
        </div>
        <div>
          <span>兼容性</span>
          <strong>{rolloutCompatibilityIssues}</strong>
        </div>
        <div>
          <span>将修复</span>
          <strong>{selectedRepairableCount}</strong>
        </div>
        <div>
          <span>归档会话</span>
          <strong>{response.summary.archivedThreads}</strong>
        </div>
      </div>

      <div className="catalog-db-path">
        <span>catalog DB</span>
        <code>{response.catalogDbPath ?? "未发现 codex-dev.db"}</code>
      </div>

      <div className="catalog-repair-list">
        {repairRows.length > 0 ? (
          repairRows.map((row) => {
            const repairable = hasRepairableCatalogIssue(row);
            return (
              <label className="catalog-repair-row" key={row.threadId}>
                <input
                  aria-label={`选择会话修复：${row.displayTitle}`}
                  type="checkbox"
                  checked={selectedIds.includes(row.threadId)}
                  disabled={disabled || !repairable}
                  onChange={() => onToggleSelected(row.threadId)}
                />
                <span className="catalog-repair-copy">
                  <strong>会话：{row.displayTitle}</strong>
                  <small>
                    {lifecycleLabel(row.lifecycle)}
                    {row.projectName ? ` / 项目：${row.projectName}` : ""}
                    {row.fileProvider ? ` / ${row.fileProvider}` : ""}
                    {" / "}
                    {formatDate(row.updatedAtMs)}
                  </small>
                </span>
                <span className="catalog-repair-codes">
                  {row.repairCodes.map((code) => (
                    <span key={code}>{catalogRepairCodeLabel(code)}</span>
                  ))}
                </span>
              </label>
            );
          })
        ) : (
          <div className="catalog-repair-empty">当前没有需要修复的 Codex 会话可见性问题。</div>
        )}
      </div>

      <div className="catalog-repair-actions">
        <button className="secondary-button" type="button" disabled={!canPreview} onClick={onPreview}>
          预览会话修复
        </button>
        <button className="primary-button" type="button" disabled={!canApply} onClick={onApply}>
          <ShieldCheck aria-hidden="true" size={17} />
          确认修复会话
        </button>
      </div>

      {preview ? <CatalogRepairResultPanel result={preview} /> : null}
    </section>
  );
}

function CatalogRepairResultPanel({ result }: { result: CatalogRepairResult }) {
  return (
    <section className="catalog-repair-result" aria-label={`将修复 ${result.changedThreads.length} 条会话可见性记录`}>
      <div>
        <h4>将修复 {result.changedThreads.length} 条会话可见性记录</h4>
        <p>{result.dryRun ? "dry-run 预览未写入数据库。" : "已写入 Codex 会话可见性元数据。"}</p>
      </div>
      {result.plannedChanges.length > 0 ? (
        <div className="catalog-change-list">
          {result.plannedChanges.map((change) => (
            <article key={`${change.threadId}-${change.action}`}>
              <strong>{change.displayTitle}</strong>
              <span>{change.cwd}</span>
              <small>
                {change.sourceKind}
                {change.modelProvider ? ` / ${change.modelProvider}` : ""}
              </small>
            </article>
          ))}
        </div>
      ) : null}
      {result.backupDir ? (
        <div className="result-line">
          <strong>备份目录</strong>
          <span>{result.backupDir}</span>
        </div>
      ) : null}
    </section>
  );
}

function ResultPanel({ title, result }: { title: string; result: MigrationResult }) {
  return (
    <section className="result-panel" aria-label={title}>
      <div>
        <h3>{title}</h3>
        <p>{result.dryRun ? "这是 dry-run 预览，没有写入文件。" : "已完成写入，备份路径如下。"}</p>
      </div>
      {result.backupDir ? (
        <div className="result-line">
          <strong>备份目录</strong>
          <span>{result.backupDir}</span>
        </div>
      ) : null}
      {result.changedThreads.length > 0 ? (
        <div className="result-line">
          <strong>会话 ID</strong>
          <ul>
            {result.changedThreads.map((threadId) => (
              <li key={threadId}>{threadId}</li>
            ))}
          </ul>
        </div>
      ) : null}
      {result.plannedRepairs.length > 0 ? (
        <div className="repair-list">
          {result.plannedRepairs.map((repair) => (
            <span key={`${repair.threadId}-${repair.code}`}>{repair.message}</span>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function CompletionNoticePanel({
  notice,
  loading,
  desktopActions,
  onRestartCodex,
  onActionError
}: {
  notice: CompletionNotice;
  loading: LoadingState;
  desktopActions: DesktopActions;
  onRestartCodex: () => void;
  onActionError: (message: string) => void;
}) {
  async function handleCopyBackup() {
    if (!notice.backupDir) {
      return;
    }
    try {
      await desktopActions.copyText(notice.backupDir);
    } catch (caught) {
      onActionError(`复制备份路径失败：${String(caught)}`);
    }
  }

  async function handleOpenBackup() {
    if (!notice.backupDir) {
      return;
    }
    try {
      await desktopActions.openPath(notice.backupDir);
    } catch (caught) {
      onActionError(`打开备份目录失败：${String(caught)}`);
    }
  }

  return (
    <section className="completion-notice" role="status" aria-live="polite">
      <span className="completion-icon">
        <CheckCircle2 aria-hidden="true" size={18} />
      </span>
      <div className="completion-copy">
        <strong>{notice.message}</strong>
        {notice.backupDir ? <span>{notice.backupDir}</span> : <span>本次操作未生成备份目录。</span>}
        {notice.restartResult ? (
          <span className={notice.restartResult.restarted ? "restart-result" : "restart-result warning"}>
            {notice.restartResult.restarted ? "已重启 Codex" : "请手动重启 Codex"}
          </span>
        ) : null}
      </div>
      {notice.backupDir || notice.restartTargetProvider ? (
        <div className="completion-actions">
          {notice.restartTargetProvider ? (
            <button
              className="ghost-button restart-button"
              type="button"
              disabled={loading !== "idle"}
              onClick={onRestartCodex}
            >
              <Power aria-hidden="true" size={15} />
              {loading === "restart" ? "正在重启 Codex" : "切换并重启 Codex"}
            </button>
          ) : null}
          {notice.backupDir ? (
            <>
              <button className="ghost-button" type="button" onClick={handleCopyBackup}>
                <Copy aria-hidden="true" size={15} />
                复制备份路径
              </button>
              <button className="ghost-button" type="button" onClick={handleOpenBackup}>
                <FolderOpen aria-hidden="true" size={15} />
                打开备份目录
              </button>
            </>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function ErrorPanel({ error }: { error: DisplayError }) {
  return (
    <section className="error-panel" role="alert">
      <strong>{error.message}</strong>
      {error.operation ? (
        <div className="error-line">
          <span>失败步骤</span>
          <code>{error.operation}</code>
        </div>
      ) : null}
      {error.backupDir ? (
        <div className="error-line">
          <span>可恢复备份</span>
          <code>{error.backupDir}</code>
        </div>
      ) : null}
    </section>
  );
}

function ConfirmMigrationDialog({
  migrationCount,
  targetProvider,
  onCancel,
  onConfirm
}: {
  migrationCount: number;
  targetProvider: string;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="modal-backdrop">
      <section aria-labelledby="confirm-migration-title" aria-modal="true" className="confirm-dialog" role="dialog">
        <div className="confirm-dialog-header">
          <span className="confirm-dialog-icon">
            <ShieldCheck aria-hidden="true" size={20} />
          </span>
          <div>
            <p className="eyebrow">写入前确认</p>
            <h3 id="confirm-migration-title">确认迁移</h3>
          </div>
        </div>
        <div className="confirm-dialog-body">
          <p>将迁移 {migrationCount} 个会话</p>
          <p>
            目标 provider：<strong>{targetProvider}</strong>
          </p>
          <p>写入前会创建备份，迁移失败时可按备份目录恢复。</p>
        </div>
        <div className="confirm-dialog-actions">
          <button className="secondary-button" type="button" onClick={onCancel}>
            取消
          </button>
          <button className="primary-button" type="button" onClick={onConfirm}>
            <ShieldCheck aria-hidden="true" size={17} />
            确认迁移
          </button>
        </div>
      </section>
    </div>
  );
}

const loadingCopy: Record<Exclude<LoadingState, "idle">, { title: string; body: string; note: string }> = {
  scan: {
    title: "正在扫描",
    body: "正在读取 Codex 会话目录",
    note: "完成后会自动刷新 provider 和会话列表。"
  },
  preview: {
    title: "正在预览",
    body: "正在检查选中会话的迁移计划",
    note: "这一步不会写入文件。"
  },
  apply: {
    title: "正在迁移",
    body: "正在创建备份并写入会话",
    note: "请保持应用打开，完成后会自动刷新列表。"
  },
  catalogPreview: {
    title: "正在预览修复",
    body: "正在检查 Codex 会话可见性修复计划",
    note: "这一步不会写入 Codex 会话可见性元数据。"
  },
  catalogApply: {
    title: "正在修复会话",
    body: "正在创建备份并写入 Codex 会话可见性元数据",
    note: "请保持 Codex 退出，完成后会刷新修复状态。"
  },
  delete: {
    title: "正在删除",
    body: "正在创建备份并删除已归档会话",
    note: "请保持应用打开，完成后会更新会话列表。"
  },
  archive: {
    title: "正在归档",
    body: "正在创建备份并移动会话文件",
    note: "请保持应用打开，完成后会自动刷新列表。"
  },
  activate: {
    title: "正在激活",
    body: "正在创建备份并恢复会话文件",
    note: "请保持应用打开，完成后会自动刷新列表。"
  },
  restart: {
    title: "正在重启",
    body: "正在切换 Codex provider 并重启应用",
    note: "请等待系统完成关闭和重新打开。"
  }
};

function OperationLoadingDialog({ loading }: { loading: Exclude<LoadingState, "idle"> }) {
  const copy = loadingCopy[loading];
  const titleId = `operation-loading-${loading}-title`;
  return (
    <div className="modal-backdrop">
      <section aria-labelledby={titleId} aria-modal="true" className="confirm-dialog loading-dialog" role="dialog">
        <div className="confirm-dialog-header">
          <span className="confirm-dialog-icon loading-dialog-icon">
            <RefreshCw aria-hidden="true" size={20} />
          </span>
          <div>
            <p className="eyebrow">正在处理</p>
            <h3 id={titleId}>{copy.title}</h3>
          </div>
        </div>
        <div className="confirm-dialog-body">
          <p>{copy.body}</p>
          <p>{copy.note}</p>
        </div>
      </section>
    </div>
  );
}

function ConfirmRestartDialog({
  targetProvider,
  onCancel,
  onConfirm
}: {
  targetProvider: string;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="modal-backdrop">
      <section aria-labelledby="confirm-restart-title" aria-modal="true" className="confirm-dialog" role="dialog">
        <div className="confirm-dialog-header">
          <span className="confirm-dialog-icon">
            <Power aria-hidden="true" size={20} />
          </span>
          <div>
            <p className="eyebrow">重启前确认</p>
            <h3 id="confirm-restart-title">切换并重启 Codex</h3>
          </div>
        </div>
        <div className="confirm-dialog-body">
          <p>
            目标 provider：<strong>{targetProvider}</strong>
          </p>
          <p>会先更新 Codex 配置，然后尝试关闭并重新打开 Codex 桌面应用。</p>
          <p>请确认当前 Codex 里没有需要保留的未完成输入。</p>
        </div>
        <div className="confirm-dialog-actions">
          <button className="secondary-button" type="button" onClick={onCancel}>
            取消
          </button>
          <button className="primary-button" type="button" onClick={onConfirm}>
            <Power aria-hidden="true" size={17} />
            确认重启
          </button>
        </div>
      </section>
    </div>
  );
}

function SessionDetailsDialog({
  row,
  targetProvider,
  onClose
}: {
  row: ThreadRow;
  targetProvider: string;
  onClose: () => void;
}) {
  const titleId = `session-details-${row.threadId}`;
  return (
    <div className="modal-backdrop">
      <section aria-labelledby={titleId} aria-modal="true" className="details-dialog" role="dialog">
        <div className="confirm-dialog-header">
          <span className="confirm-dialog-icon details">
            <FileText aria-hidden="true" size={20} />
          </span>
          <div>
            <p className="eyebrow">高级信息</p>
            <h3 id={titleId}>{row.displayName} 高级信息</h3>
          </div>
        </div>
        <dl className="details-grid">
          <div>
            <dt>ID</dt>
            <dd>{row.threadId}</dd>
          </div>
          <div>
            <dt>Provider</dt>
            <dd>
              {row.fileProvider ?? "未知"} -&gt; {targetProvider}
            </dd>
          </div>
          <div>
            <dt>类型</dt>
            <dd>{lifecycleLabel(row.lifecycle)}</dd>
          </div>
          <div>
            <dt>项目</dt>
            <dd>{row.projectPath ?? row.projectName ?? "未知"}</dd>
          </div>
          <div>
            <dt>问题</dt>
            <dd>{row.issueCodes.length > 0 ? row.issueCodes.join(", ") : "无"}</dd>
          </div>
          <div>
            <dt>文件</dt>
            <dd>{row.path}</dd>
          </div>
        </dl>
        <div className="confirm-dialog-actions">
          <button className="primary-button" type="button" onClick={onClose}>
            关闭
          </button>
        </div>
      </section>
    </div>
  );
}

function SessionTranscriptDialog({
  row,
  transcript,
  loading,
  error,
  onClose
}: {
  row: ThreadRow;
  transcript: SessionTranscript | null;
  loading: boolean;
  error: DisplayError | null;
  onClose: () => void;
}) {
  const titleId = `session-transcript-${row.threadId}`;
  const turns = transcript?.turns ?? [];
  return (
    <div className="modal-backdrop">
      <section aria-labelledby={titleId} aria-modal="true" className="details-dialog transcript-dialog" role="dialog">
        <div className="confirm-dialog-header">
          <span className="confirm-dialog-icon transcript">
            <MessageSquareText aria-hidden="true" size={20} />
          </span>
          <div className="transcript-dialog-title-wrap">
            <p className="eyebrow">会话记录</p>
            <h3 className="transcript-dialog-title" id={titleId}>
              {row.displayName} 会话记录
            </h3>
          </div>
        </div>

        <div className="transcript-list">
          {!loading && !error && transcript && transcript.omittedTurns > 0 ? (
            <div className="transcript-omitted">
              记录较多，已优先展示最近 {transcript.turns.length} 条，前面 {transcript.omittedTurns} 条已省略。
            </div>
          ) : null}
          {loading ? (
            <div className="transcript-state">
              <RefreshCw aria-hidden="true" size={18} />
              正在读取会话记录...
            </div>
          ) : null}
          {!loading && error ? (
            <div className="transcript-state error" role="alert">
              {error.message}
            </div>
          ) : null}
          {!loading && !error && turns.length === 0 ? (
            <div className="transcript-state">这条会话暂时没有可展示的用户/助手消息。</div>
          ) : null}
          {!loading && !error
            ? turns.map((turn) => (
                <article className={`transcript-turn ${turn.role}`} key={`${turn.index}-${turn.role}`}>
                  <div className="transcript-turn-meta">
                    <span>{transcriptRoleLabel(turn.role)}</span>
                    {turn.timestamp ? <time>{formatTranscriptTime(turn.timestamp)}</time> : null}
                  </div>
                  <p className="transcript-text">{turn.text}</p>
                </article>
              ))
            : null}
        </div>

        <div className="confirm-dialog-actions">
          <button className="primary-button" type="button" onClick={onClose}>
            关闭
          </button>
        </div>
      </section>
    </div>
  );
}

function ConfirmLifecycleDialog({
  action,
  row,
  onCancel,
  onConfirm
}: {
  action: "archive" | "activate";
  row: ThreadRow;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const isArchive = action === "archive";
  const title = isArchive ? "确认归档会话" : "确认激活会话";
  const actionText = isArchive ? "归档" : "激活";
  const scopeText = isArchive ? "活跃会话" : "已归档会话";
  return (
    <div className="modal-backdrop">
      <section aria-labelledby="confirm-lifecycle-title" aria-modal="true" className="confirm-dialog" role="dialog">
        <div className="confirm-dialog-header">
          <span className="confirm-dialog-icon">
            {isArchive ? (
              <Archive aria-hidden="true" size={20} />
            ) : (
              <ArchiveRestore aria-hidden="true" size={20} />
            )}
          </span>
          <div>
            <p className="eyebrow">更新前确认</p>
            <h3 id="confirm-lifecycle-title">{title}</h3>
          </div>
        </div>
        <div className="confirm-dialog-body">
          <p>
            将{actionText} 1 个{scopeText}
          </p>
          <p>
            会话：<strong>{row.displayName}</strong>
          </p>
          <p>会先创建备份，然后移动会话文件并更新本地状态库。</p>
        </div>
        <div className="confirm-dialog-actions">
          <button className="secondary-button" type="button" onClick={onCancel}>
            取消
          </button>
          <button className="primary-button" type="button" onClick={onConfirm}>
            {isArchive ? (
              <Archive aria-hidden="true" size={17} />
            ) : (
              <ArchiveRestore aria-hidden="true" size={17} />
            )}
            确认{actionText}
          </button>
        </div>
      </section>
    </div>
  );
}

function ConfirmDeleteArchivedDialog({
  deleteCount,
  onCancel,
  onConfirm
}: {
  deleteCount: number;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="modal-backdrop">
      <section aria-label="Delete archived sessions" aria-modal="true" className="confirm-dialog" role="dialog">
        <div className="confirm-dialog-header">
          <span className="confirm-dialog-icon danger">
            <AlertTriangle aria-hidden="true" size={20} />
          </span>
          <div>
            <p className="eyebrow">删除前确认</p>
            <h3>确认删除已归档会话</h3>
          </div>
        </div>
        <div className="confirm-dialog-body">
          <p>将删除 {deleteCount} 个已归档会话</p>
          <p>会先创建备份，然后从 archived_sessions 和本地状态库移除对应记录。</p>
          <p>活跃会话不会被删除。</p>
        </div>
        <div className="confirm-dialog-actions">
          <button className="secondary-button" type="button" onClick={onCancel}>
            取消
          </button>
          <button className="primary-button danger-primary-button" type="button" data-testid="confirm-delete-archived" onClick={onConfirm}>
            <Trash2 aria-hidden="true" size={17} />
            确认删除
          </button>
        </div>
      </section>
    </div>
  );
}

function summaryTitle(scanResponse: ScanResponse | null, selectedCount: number) {
  if (!scanResponse) {
    return "准备扫描 Codex 会话";
  }
  return `准备迁移 ${selectedCount} 个会话`;
}

function englishSummaryTitle(scanResponse: ScanResponse | null, selectedCount: number) {
  if (!scanResponse) {
    return "Prepare to Scan Codex Sessions";
  }
  return `Prepare to Migrate ${selectedCount} Sessions`;
}

function defaultSourceForScan(scanResponse: ScanResponse) {
  const sourceProviders = new Set(scanResponse.providerOptions.sourceProviders);
  const activeProvider = scanResponse.dashboard.rows.find(
    (row) => row.lifecycle === "active" && row.fileProvider && sourceProviders.has(row.fileProvider)
  )?.fileProvider;
  if (activeProvider) {
    return activeProvider;
  }
  const configProvider = scanResponse.providerOptions.currentConfigProvider;
  if (configProvider && sourceProviders.has(configProvider)) {
    return configProvider;
  }
  return ALL_SOURCES;
}

function sourceProviderOptions(scanResponse: ScanResponse, selectedSourceProvider: string) {
  const providers = new Set(scanResponse.providerOptions.sourceProviders);
  if (selectedSourceProvider !== ALL_SOURCES) {
    providers.add(selectedSourceProvider);
  }
  const counts = scanResponse.dashboard.rows.reduce<Record<string, number>>((providerCounts, row) => {
    if (row.fileProvider) {
      providerCounts[row.fileProvider] = (providerCounts[row.fileProvider] ?? 0) + 1;
    }
    return providerCounts;
  }, {});
  return Array.from(providers).map((provider) => ({
    value: provider,
    label: `${provider} (${counts[provider] ?? 0})`
  }));
}

function emptyVisibleRowsText(sourceProvider: string) {
  return sourceProvider === ALL_SOURCES
    ? "当前来源 provider 下没有可见会话。"
    : "该来源 provider 已无待迁移会话。";
}

function threadIdsForSource(rows: ThreadRow[], sourceProvider: string) {
  return rows
    .filter((row) => sourceProvider === ALL_SOURCES || row.fileProvider === sourceProvider)
    .map((row) => row.threadId);
}

function defaultCatalogRepairThreadIds(response: CatalogRepairScanResponse) {
  return response.rows.filter((row) => row.selectedByDefault && hasRepairableCatalogIssue(row)).map((row) => row.threadId);
}

function hasRepairableCatalogIssue(row: CatalogRepairRow) {
  return row.repairCodes.some((code) =>
    [
      "missing_catalog_entry",
      "catalog_cwd_mismatch",
      "catalog_title_stale",
      "missing_session_index",
      "missing_state_entry",
      "state_rollout_path_mismatch",
      "state_cwd_mismatch",
      "rollout_internal_metadata_passthrough"
    ].includes(code)
  );
}

function compareSessionRows(left: ThreadRow, right: ThreadRow) {
  return (
    lifecycleRank(left) - lifecycleRank(right) ||
    right.updatedAtMs - left.updatedAtMs ||
    left.threadId.localeCompare(right.threadId)
  );
}

function lifecycleRank(row: ThreadRow) {
  return row.lifecycle === "archived" ? 1 : 0;
}

function lifecycleLabel(lifecycle: ThreadRow["lifecycle"]) {
  return lifecycle === "archived" ? "已归档" : "活跃";
}

function transcriptRoleLabel(role: string) {
  const labels: Record<string, string> = {
    user: "用户",
    assistant: "助手",
    system: "系统",
    tool: "工具",
    other: "其他"
  };
  return labels[role] ?? "其他";
}

function formatTranscriptTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit"
  }).format(date);
}

function issueSummary(issueCodes: string[]) {
  if (issueCodes.length === 0) {
    return "未发现需要修复的可见性问题。";
  }
  const labels: Record<string, string> = {
    bom_present: "包含 UTF-8 BOM",
    provider_mismatch: "provider 与当前配置不一致",
    missing_index: "缺少 session_index 记录",
    missing_state_entry: "缺少 sqlite 可见性记录",
    state_provider_mismatch: "sqlite provider 不一致",
    archived_state: "sqlite 中处于归档状态"
  };
  return issueCodes.map((code) => labels[code] ?? code).join("，");
}

function formatDate(value: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit"
  }).format(new Date(value));
}

function requestKey(request: MigrationRequest | CatalogRepairRequest) {
  return JSON.stringify(request);
}

function catalogRepairCodeLabel(code: string) {
  const labels: Record<string, string> = {
    missing_catalog_entry: "缺失 catalog",
    catalog_db_missing: "缺失 codex-dev.db",
    catalog_cwd_mismatch: "项目路径不一致",
    catalog_title_stale: "标题待补齐",
    missing_session_index: "缺失 session_index",
    missing_state_entry: "缺失 state 记录",
    state_rollout_path_mismatch: "state 路径待修复",
    state_cwd_mismatch: "state 项目路径待修复",
    rollout_internal_metadata_passthrough: "会话兼容性待修复"
  };
  return labels[code] ?? code;
}

function countIssues(rows: ThreadRow[]) {
  return rows.reduce<Record<string, number>>((counts, row) => {
    for (const code of row.issueCodes) {
      counts[code] = (counts[code] ?? 0) + 1;
    }
    return counts;
  }, {});
}

function errorMessage(caught: unknown): DisplayError {
  if (typeof caught === "object" && caught && "message" in caught) {
    const commandError = caught as CommandError;
    return {
      message: localizedCommandErrorMessage(commandError),
      operation: commandError.operation,
      backupDir: commandError.backupDir
    };
  }
  return { message: String(caught) };
}

function localizedCommandErrorMessage(error: CommandError) {
  const fallback = String(error.message);
  const messages: Record<string, string> = {
    codex_home_missing: "找不到 Codex 目录，请确认路径是否正确。",
    sessions_missing: "找不到 sessions 目录，请确认这是 Codex 数据目录。",
    no_sessions_found: "没有找到 Codex 会话，请确认 sessions 或 archived_sessions 目录里存在 rollout-*.jsonl 文件。",
    target_provider_required: "请选择目标 provider。",
    no_session_selected: "请至少选择一个要处理的会话。",
    no_threads_selected: "请至少选择一个要修复的会话。",
    selected_thread_missing: "选中的会话已经不存在，请重新扫描。",
    codex_process_running: "请先退出 Codex，再修复 Codex 会话可见性。",
    catalog_db_missing: "找不到 Codex 会话可见性数据库，请先启动一次 Codex 让它生成 catalog。",
    catalog_schema_unsupported: "Codex 会话可见性数据库结构暂不支持自动修复。",
    catalog_insert_failed: "写入 Codex 会话可见性元数据失败，请确认 Codex 已退出后重试。",
    catalog_update_failed: "更新 Codex 会话可见性元数据失败，请确认 Codex 已退出后重试。",
    delete_requires_archived_sessions: "只能删除已归档会话；活跃会话不会被删除。",
    archive_requires_active_sessions: "只能归档活跃会话。",
    activate_requires_archived_sessions: "只能激活已归档会话。",
    target_session_exists: "目标会话文件已存在，请重新扫描或检查会话目录。",
    invalid_utf8: "会话文件不是有效的 UTF-8，暂时无法安全迁移。",
    invalid_jsonl: "会话 JSONL 无法解析，暂时无法安全迁移。",
    missing_session_meta: "会话文件缺少 session_meta 开头，暂时无法安全迁移。",
    missing_payload: "会话元数据缺少 payload，暂时无法安全迁移。",
    missing_thread_id: "会话元数据缺少 ID，暂时无法安全迁移。",
    provider_marker_missing: "会话文件中找不到 model_provider 标记，暂时无法迁移。",
    sqlite_open_failed: "sqlite 数据库打开失败，请关闭 Codex 后重试。",
    sqlite_busy_timeout_failed: "sqlite 数据库正忙，请关闭 Codex 后重试。",
    sqlite_query_failed: "读取 sqlite 可见性元数据失败。",
    sqlite_update_failed: "更新 sqlite 可见性元数据失败。",
    sqlite_insert_failed: "写入 sqlite 可见性元数据失败。",
    sqlite_delete_failed: "删除 sqlite 归档会话记录失败。",
    backup_directory_collision: "创建备份目录失败，请稍后重试。",
    post_backup_write_failed: fallback,
    io_error: fallback
  };
  return messages[error.code] ?? fallback;
}
