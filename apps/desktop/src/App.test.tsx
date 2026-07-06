import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, test, vi } from "vitest";
import App from "./App";
import type { MigrationApi } from "./domain/migrationApi";
import type { CatalogRepairScanResponse, MigrationResult, ScanResponse, SessionTranscript } from "./domain/session";

const fixtureCodexHome = "D:\\Codex\\fixture\\.codex";
const activeThreadId = "019eca3b-941d-7340-9b14-328c635a6523";
const archivedThreadId = "019ec94d-720d-7a12-a379-28c8042bc6b4";

afterEach(() => {
  cleanup();
});

function fakeDesktopActions() {
  return {
    openPath: vi.fn().mockResolvedValue(undefined),
    copyText: vi.fn().mockResolvedValue(undefined)
  };
}

const scanResponse: ScanResponse = {
  dashboard: {
    codexHome: fixtureCodexHome,
    totalThreads: 2,
    problemThreads: 2,
    issueCounts: { provider_mismatch: 2 },
    rows: [
      {
        threadId: activeThreadId,
        shortId: "019eca3b",
        displayName: "活跃 provider 会话",
        projectName: null,
        projectPath: null,
        path: `${fixtureCodexHome}\\sessions\\rollout-a.jsonl`,
        fileProvider: "funai",
        configProvider: "yihubangg",
        lifecycle: "active",
        issueCodes: ["provider_mismatch"],
        severity: 70,
        canMigrate: true,
        suggestedActionCode: "migrate_provider",
        suggestedActionValues: { source: "funai", target: "yihubangg" },
        updatedAtMs: 1781484460000
      },
      {
        threadId: archivedThreadId,
        shortId: "019ec94d",
        displayName: "归档 provider 会话",
        projectName: null,
        projectPath: null,
        path: `${fixtureCodexHome}\\archived_sessions\\rollout-b.jsonl`,
        fileProvider: "gmn",
        configProvider: "yihubangg",
        lifecycle: "archived",
        issueCodes: ["provider_mismatch"],
        severity: 70,
        canMigrate: true,
        suggestedActionCode: "migrate_provider",
        suggestedActionValues: { source: "gmn", target: "yihubangg" },
        updatedAtMs: 1781484400000
      }
    ]
  },
  providerOptions: {
    currentConfigProvider: "yihubangg",
    sourceProviders: ["funai", "gmn"],
    targetProviders: [
      { value: "yihubangg", label: "yihubangg（当前配置，推荐）", kind: "config", recommended: true },
      { value: "funai", label: "funai", kind: "discovered", recommended: false },
      { value: "gmn", label: "gmn", kind: "discovered", recommended: false }
    ]
  },
  configProvider: "yihubangg"
};

const scanResponseAfterMigration: ScanResponse = {
  ...scanResponse,
  dashboard: {
    ...scanResponse.dashboard,
    problemThreads: 1,
    issueCounts: { provider_mismatch: 1 },
    rows: [
      {
        ...scanResponse.dashboard.rows[0],
        fileProvider: "yihubangg",
        issueCodes: [],
        severity: 0,
        canMigrate: false,
        suggestedActionCode: "none",
        suggestedActionValues: {}
      },
      scanResponse.dashboard.rows[1]
    ]
  },
  providerOptions: {
    ...scanResponse.providerOptions,
    sourceProviders: ["yihubangg", "gmn"]
  }
};

const scanResponseAfterArchive: ScanResponse = {
  ...scanResponse,
  dashboard: {
    ...scanResponse.dashboard,
    rows: [
      {
        ...scanResponse.dashboard.rows[0],
        lifecycle: "archived",
        path: `${fixtureCodexHome}\\archived_sessions\\rollout-a.jsonl`
      },
      scanResponse.dashboard.rows[1]
    ]
  }
};

const scanResponseAfterActivate: ScanResponse = {
  ...scanResponse,
  dashboard: {
    ...scanResponse.dashboard,
    rows: [
      scanResponse.dashboard.rows[0],
      {
        ...scanResponse.dashboard.rows[1],
        lifecycle: "active",
        path: `${fixtureCodexHome}\\sessions\\2026\\06\\17\\rollout-b.jsonl`
      }
    ]
  }
};

const catalogRepairScanResponse: CatalogRepairScanResponse = {
  catalogDbPath: `${fixtureCodexHome}\\sqlite\\codex-dev.db`,
  summary: {
    totalThreads: 2,
    missingCatalogEntries: 1,
    selectedByDefault: 1,
    archivedThreads: 1
  },
  rows: [
    {
      threadId: activeThreadId,
      displayTitle: "活跃 provider 会话",
      lifecycle: "active",
      projectName: null,
      projectPath: null,
      path: `${fixtureCodexHome}\\sessions\\rollout-a.jsonl`,
      fileProvider: "funai",
      repairCodes: ["missing_catalog_entry"],
      selectedByDefault: true,
      updatedAtMs: 1781484460000
    },
    {
      threadId: archivedThreadId,
      displayTitle: "归档 provider 会话",
      lifecycle: "archived",
      projectName: null,
      projectPath: null,
      path: `${fixtureCodexHome}\\archived_sessions\\rollout-b.jsonl`,
      fileProvider: "gmn",
      repairCodes: ["missing_catalog_entry"],
      selectedByDefault: false,
      updatedAtMs: 1781484400000
    }
  ]
};

const activeTranscript: SessionTranscript = {
  threadId: activeThreadId,
  title: "活跃 provider 会话",
  path: `${fixtureCodexHome}\\sessions\\rollout-a.jsonl`,
  omittedTurns: 0,
  turns: [
    {
      role: "user",
      text: "帮我把这个会话切到 yihubangg",
      timestamp: "2026-06-15T01:01:00.000Z",
      index: 0
    },
    {
      role: "assistant",
      text: "可以，我会先检查会话 provider。",
      timestamp: "2026-06-15T01:02:00.000Z",
      index: 1
    }
  ]
};

function fakeApi(): MigrationApi {
  return {
    scanCodexHome: vi.fn().mockResolvedValue(scanResponse),
    scanCodexCatalogRepair: vi.fn().mockResolvedValue(catalogRepairScanResponse),
    previewProviderMigration: vi.fn().mockResolvedValue({
      changedThreads: [activeThreadId],
      plannedRepairs: [
        {
          threadId: activeThreadId,
          code: "update_provider",
          message: "更新会话文件中的 model_provider"
        }
      ],
      backupDir: null,
      dryRun: true
    }),
    previewCodexCatalogRepair: vi.fn().mockResolvedValue({
      changedThreads: [activeThreadId],
      plannedChanges: [
        {
          threadId: activeThreadId,
          action: "insert_catalog_entry",
          displayTitle: "活跃 provider 会话",
          cwd: "D:\\work",
          sourceKind: "vscode",
          modelProvider: "funai"
        }
      ],
      backupDir: null,
      dryRun: true
    }),
    applyProviderMigration: vi.fn().mockResolvedValue({
      changedThreads: [activeThreadId],
      plannedRepairs: [],
      backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-120000`,
      dryRun: false
    }),
    applyCodexCatalogRepair: vi.fn().mockResolvedValue({
      changedThreads: [activeThreadId],
      plannedChanges: [],
      backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260706-170000`,
      dryRun: false
    }),
    previewDeleteArchivedSessions: vi.fn().mockResolvedValue({
      deletedThreads: [archivedThreadId],
      backupDir: null,
      dryRun: true
    }),
    applyDeleteArchivedSessions: vi.fn().mockResolvedValue({
      deletedThreads: [archivedThreadId],
      backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-130000`,
      dryRun: false
    }),
    applyArchiveSessions: vi.fn().mockResolvedValue({
      changedThreads: [activeThreadId],
      backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-150000`
    }),
    applyActivateSessions: vi.fn().mockResolvedValue({
      changedThreads: [archivedThreadId],
      backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-160000`
    }),
    switchProviderAndRestart: vi.fn().mockResolvedValue({
      configuredProvider: "yihubangg",
      previousProvider: "funai",
      configBackupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-140000`,
      restartAttempted: true,
      restarted: true,
      restartMessage: "Codex 已按新 provider 配置重新启动。"
    }),
    readSessionTranscript: vi.fn().mockResolvedValue(activeTranscript)
  };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}

async function renderWorkflow(api = fakeApi()) {
  const user = userEvent.setup();
  render(
    <App
      migrationApi={api}
      resolveDefaultCodexHome={() => Promise.resolve(fixtureCodexHome)}
      showStartupSplash={false}
    />
  );
  await screen.findByDisplayValue(fixtureCodexHome);
  return { api, user };
}

async function renderWorkflowWithDesktopActions(api = fakeApi(), desktopActions = fakeDesktopActions()) {
  const user = userEvent.setup();
  render(
    <App
      migrationApi={api}
      desktopActions={desktopActions}
      resolveDefaultCodexHome={() => Promise.resolve(fixtureCodexHome)}
      showStartupSplash={false}
    />
  );
  await screen.findByDisplayValue(fixtureCodexHome);
  return { api, desktopActions, user };
}

async function expectBlockingLoadingDialog(name: string, text: string) {
  const dialog = await screen.findByRole("dialog", { name });
  expect(dialog).toHaveTextContent(text);
  expect(within(dialog).queryByRole("button")).not.toBeInTheDocument();
  return dialog;
}

test("scan shows a blocking loading dialog while reading sessions", async () => {
  const api = fakeApi();
  const pendingScan = deferred<ScanResponse>();
  vi.mocked(api.scanCodexHome).mockReturnValueOnce(pendingScan.promise);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));

  await expectBlockingLoadingDialog("正在扫描", "正在读取 Codex 会话目录");
  expect(screen.getByRole("button", { name: /正在扫描/ })).toBeDisabled();

  pendingScan.resolve(scanResponse);

  await waitFor(() => {
    expect(screen.queryByRole("dialog", { name: "正在扫描" })).not.toBeInTheDocument();
  });
  expect(await screen.findByText("活跃 provider 会话")).toBeInTheDocument();
});

test("app shows the GSAP startup splash by default", async () => {
  document.body.insertAdjacentHTML("afterbegin", '<div id="preload-splash">Preload splash</div>');

  render(
    <App
      migrationApi={fakeApi()}
      resolveDefaultCodexHome={() => Promise.resolve(fixtureCodexHome)}
      startupSplashDurationMs={0}
    />
  );

  expect(screen.getByRole("status", { name: "AI Session Migrator 启动闪屏" })).toHaveTextContent(
    "Codex 会话迁移助手"
  );
  expect(document.getElementById("preload-splash")).not.toBeInTheDocument();
});

test("scan shows active sessions before archived sessions with lifecycle badges", async () => {
  const { user } = await renderWorkflow();

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  expect(screen.getByLabelText("来源 provider")).toHaveValue("funai");
  const sourceSelect = screen.getByLabelText("来源 provider");
  expect(within(sourceSelect).getByRole("option", { name: "全部 provider (2)" })).toHaveValue("__all__");
  expect(within(sourceSelect).getByRole("option", { name: "funai (1)" })).toHaveValue("funai");
  expect(within(sourceSelect).getByRole("option", { name: "gmn (1)" })).toHaveValue("gmn");
  expect(screen.getByText("1 个可见，1 个已选")).toBeInTheDocument();
  expect(screen.queryByText("状态")).not.toBeInTheDocument();
  expect(screen.queryByText(/可迁移/)).not.toBeInTheDocument();
  expect(screen.queryByRole("article", { name: "归档 provider 会话" })).not.toBeInTheDocument();

  await user.selectOptions(screen.getByLabelText("来源 provider"), "__all__");
  const activeRow = await screen.findByRole("article", { name: "活跃 provider 会话" });
  const archivedRow = screen.getByRole("article", { name: "归档 provider 会话" });

  expect(activeRow).toHaveClass("session-row-active");
  expect(archivedRow).toHaveClass("session-row-archived");
  expect(within(activeRow).getByText("活跃")).toBeInTheDocument();
  expect(within(archivedRow).getByText("已归档")).toBeInTheDocument();
  expect(within(archivedRow).getByRole("button", { name: "激活" })).not.toBeDisabled();
  expect(within(archivedRow).getByRole("button", { name: "删除" })).not.toBeDisabled();
  expect(activeRow.compareDocumentPosition(archivedRow) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(screen.getByText("2 个可见，2 个已选")).toBeInTheDocument();
});

test("shows codex catalog repair action after scan", async () => {
  const { user } = await renderWorkflow();

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));

  expect(await screen.findByRole("button", { name: /修复 Codex 可见索引/ })).toBeEnabled();
});

test("session item shows the owning project beside the lifecycle badge", async () => {
  const api = fakeApi();
  vi.mocked(api.scanCodexHome).mockResolvedValueOnce({
    ...scanResponse,
    dashboard: {
      ...scanResponse.dashboard,
      rows: [
        {
          ...scanResponse.dashboard.rows[0],
          projectName: "fun-claw",
          projectPath: "D:\\dev\\AI\\AIPro\\fun-claw"
        },
        scanResponse.dashboard.rows[1]
      ]
    }
  } as ScanResponse);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));

  const activeRow = await screen.findByRole("article", { name: scanResponse.dashboard.rows[0].displayName });
  expect(within(activeRow).getByText("项目：fun-claw")).toHaveClass("project-badge");
  expect(within(activeRow).getByText("活跃")).toHaveClass("lifecycle-badge");
});

test("session list separates time and actions columns", async () => {
  const { user } = await renderWorkflow();

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await user.selectOptions(screen.getByLabelText("来源 provider"), "__all__");

  expect(screen.getByText("时间")).toBeInTheDocument();
  expect(screen.getByText("操作")).toBeInTheDocument();
  expect(screen.queryByText("时间与操作")).not.toBeInTheDocument();
});

test("advanced session details open in a dialog", async () => {
  const { user } = await renderWorkflow();

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const activeRow = await screen.findByRole("article", { name: "活跃 provider 会话" });
  await user.click(within(activeRow).getByRole("button", { name: "高级信息" }));

  const dialog = screen.getByRole("dialog", { name: "活跃 provider 会话 高级信息" });
  expect(dialog).toHaveTextContent(activeThreadId);
  expect(dialog).toHaveTextContent("funai -> yihubangg");
  expect(dialog).toHaveTextContent(`${fixtureCodexHome}\\sessions\\rollout-a.jsonl`);
  expect(within(activeRow).queryByText(`ID: ${activeThreadId}`)).not.toBeInTheDocument();

  await user.click(within(dialog).getByRole("button", { name: "关闭" }));
  expect(screen.queryByRole("dialog", { name: "活跃 provider 会话 高级信息" })).not.toBeInTheDocument();
});

test("session transcript opens in a read-only dialog", async () => {
  const api = fakeApi();
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const activeRow = await screen.findByRole("article", { name: "活跃 provider 会话" });
  await user.click(within(activeRow).getByRole("button", { name: "查看记录" }));

  await waitFor(() => {
    expect(api.readSessionTranscript).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      threadId: activeThreadId,
      path: `${fixtureCodexHome}\\sessions\\rollout-a.jsonl`
    });
  });

  const dialog = await screen.findByRole("dialog", { name: "活跃 provider 会话 会话记录" });
  expect(dialog).toHaveTextContent("帮我把这个会话切到 yihubangg");
  expect(dialog).toHaveTextContent("可以，我会先检查会话 provider。");
  expect(dialog).toHaveTextContent("用户");
  expect(dialog).toHaveTextContent("助手");

  await user.click(within(dialog).getByRole("button", { name: "关闭" }));
  expect(screen.queryByRole("dialog", { name: "活跃 provider 会话 会话记录" })).not.toBeInTheDocument();
});

test("session transcript dialog constrains very long titles", async () => {
  const longTitle =
    "你是 FunClaw 工作台委托给本机 Codex CLI 的数字员工执行器。Assignment ID: b80d162e-d783-40c7-9a3d-a2440cabb349 Metadata JSON: {\"source\":\"WORKBENCH_MESSAGE\"}";
  const api = fakeApi();
  vi.mocked(api.scanCodexHome).mockResolvedValueOnce({
    ...scanResponse,
    dashboard: {
      ...scanResponse.dashboard,
      rows: [
        {
          ...scanResponse.dashboard.rows[0],
          displayName: longTitle
        }
      ]
    }
  });
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const activeRow = await screen.findByRole("article", { name: longTitle });
  await user.click(within(activeRow).getByRole("button", { name: "查看记录" }));

  const dialog = await screen.findByRole("dialog", { name: `${longTitle} 会话记录` });
  expect(within(dialog).getByRole("heading", { name: `${longTitle} 会话记录` })).toHaveClass(
    "transcript-dialog-title"
  );
});

test("closing transcript dialog ignores stale pending responses", async () => {
  const api = fakeApi();
  const pendingTranscript = deferred<SessionTranscript>();
  vi.mocked(api.readSessionTranscript).mockReturnValueOnce(pendingTranscript.promise);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const activeRow = await screen.findByRole("article", { name: "活跃 provider 会话" });
  await user.click(within(activeRow).getByRole("button", { name: "查看记录" }));

  const loadingDialog = await screen.findByRole("dialog", { name: "活跃 provider 会话 会话记录" });
  expect(loadingDialog).toHaveTextContent("正在读取会话记录");
  await user.click(within(loadingDialog).getByRole("button", { name: "关闭" }));
  expect(screen.queryByRole("dialog", { name: "活跃 provider 会话 会话记录" })).not.toBeInTheDocument();

  pendingTranscript.resolve(activeTranscript);

  await waitFor(() => {
    expect(screen.queryByRole("dialog", { name: "活跃 provider 会话 会话记录" })).not.toBeInTheDocument();
  });
});

test("session transcript reports omitted older turns when response is capped", async () => {
  const api = fakeApi();
  vi.mocked(api.readSessionTranscript).mockResolvedValueOnce({
    ...activeTranscript,
    omittedTurns: 12,
    turns: activeTranscript.turns.slice(0, 1)
  });
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const activeRow = await screen.findByRole("article", { name: "活跃 provider 会话" });
  await user.click(within(activeRow).getByRole("button", { name: "查看记录" }));

  const dialog = await screen.findByRole("dialog", { name: "活跃 provider 会话 会话记录" });
  expect(dialog).toHaveTextContent("记录较多，已优先展示最近 1 条，前面 12 条已省略。");
});

test("preview sends selected visible rows and target provider to the API", async () => {
  const api = fakeApi();
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await screen.findByText("活跃 provider 会话");
  await user.selectOptions(screen.getByLabelText("来源 provider"), "gmn");
  await user.click(screen.getByRole("button", { name: /预览迁移/ }));

  await waitFor(() => {
    expect(api.previewProviderMigration).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      sourceProvider: "gmn",
      targetProvider: "yihubangg",
      threadIds: [archivedThreadId]
    });
  });
});

test("confirm migration opens a dialog before applying", async () => {
  const api = fakeApi();
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await screen.findByText("活跃 provider 会话");
  await user.selectOptions(screen.getByLabelText("来源 provider"), "funai");
  await user.click(screen.getByRole("button", { name: /预览迁移/ }));
  await screen.findByText("将更新 1 个会话");
  await user.click(screen.getByRole("button", { name: /确认迁移/ }));

  const dialog = screen.getByRole("dialog", { name: "确认迁移" });
  expect(dialog).toHaveTextContent("将迁移 1 个会话");
  expect(api.applyProviderMigration).not.toHaveBeenCalled();

  await user.click(within(dialog).getByRole("button", { name: "确认迁移" }));

  await waitFor(() => {
    expect(api.applyProviderMigration).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      sourceProvider: "funai",
      targetProvider: "yihubangg",
      threadIds: [activeThreadId]
    });
  });
});

test("apply migration shows a blocking loading dialog while writing files", async () => {
  const api = fakeApi();
  const pendingMigration = deferred<MigrationResult>();
  vi.mocked(api.scanCodexHome)
    .mockResolvedValueOnce(scanResponse)
    .mockResolvedValueOnce(scanResponseAfterMigration);
  vi.mocked(api.applyProviderMigration).mockReturnValueOnce(pendingMigration.promise);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await screen.findByText("活跃 provider 会话");
  await user.selectOptions(screen.getByLabelText("来源 provider"), "funai");
  await user.click(screen.getByRole("button", { name: /预览迁移/ }));
  await screen.findByText("将更新 1 个会话");
  await user.click(screen.getByRole("button", { name: /确认迁移/ }));
  await user.click(within(screen.getByRole("dialog", { name: "确认迁移" })).getByRole("button", { name: "确认迁移" }));

  const loadingDialog = await screen.findByRole("dialog", { name: "正在迁移" });
  expect(loadingDialog).toHaveTextContent("正在创建备份并写入会话");
  expect(within(loadingDialog).queryByRole("button")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: /正在迁移/ })).toBeDisabled();

  pendingMigration.resolve({
    changedThreads: [activeThreadId],
    plannedRepairs: [],
    backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-120000`,
    dryRun: false
  });

  await waitFor(() => {
    expect(screen.queryByRole("dialog", { name: "正在迁移" })).not.toBeInTheDocument();
  });
  expect(await screen.findByRole("dialog", { name: "切换并重启 Codex" })).toBeInTheDocument();
});

test("apply migration refreshes the list and keeps completion feedback compact", async () => {
  const api = fakeApi();
  const desktopActions = fakeDesktopActions();
  vi.mocked(api.scanCodexHome)
    .mockResolvedValueOnce(scanResponse)
    .mockResolvedValueOnce(scanResponseAfterMigration);
  const { user } = await renderWorkflowWithDesktopActions(api, desktopActions);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await screen.findByText("活跃 provider 会话");
  await user.selectOptions(screen.getByLabelText("来源 provider"), "funai");
  await user.click(screen.getByRole("button", { name: /预览迁移/ }));
  await screen.findByText("将更新 1 个会话");
  await user.click(screen.getByRole("button", { name: /确认迁移/ }));
  await user.click(within(screen.getByRole("dialog", { name: "确认迁移" })).getByRole("button", { name: "确认迁移" }));

  await waitFor(() => {
    expect(api.scanCodexHome).toHaveBeenCalledTimes(2);
  });

  expect(screen.queryByText("迁移已完成")).not.toBeInTheDocument();
  expect(screen.queryByRole("article", { name: "活跃 provider 会话" })).not.toBeInTheDocument();
  expect(screen.getByLabelText("来源 provider")).toHaveValue("funai");
  expect(screen.getByText("该来源 provider 已无待迁移会话。")).toBeInTheDocument();
  expect(screen.getByText("0 个可见，0 个已选")).toBeInTheDocument();

  const completion = screen.getByRole("status");
  expect(completion).toHaveTextContent("已完成迁移 1 个会话");
  expect(completion).toHaveTextContent(`${fixtureCodexHome}\\ai-session-migrator-backup-20260617-120000`);

  await user.click(within(completion).getByRole("button", { name: "复制备份路径" }));
  expect(desktopActions.copyText).toHaveBeenCalledWith(
    `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-120000`
  );

  await user.click(within(completion).getByRole("button", { name: "打开备份目录" }));
  expect(desktopActions.openPath).toHaveBeenCalledWith(
    `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-120000`
  );
});

test("delete archived sessions requires confirmation and removes only archived rows", async () => {
  const api = fakeApi();
  const desktopActions = fakeDesktopActions();
  const { user } = await renderWorkflowWithDesktopActions(api, desktopActions);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await screen.findByText("活跃 provider 会话");
  await user.selectOptions(screen.getByLabelText("来源 provider"), "__all__");
  await user.click(screen.getByTestId("delete-archived-button"));

  const dialog = screen.getByRole("dialog", { name: /delete archived sessions/i });
  expect(dialog).toHaveTextContent("将删除 1 个已归档会话");
  expect(dialog).toHaveTextContent("会先创建备份");
  expect(api.applyDeleteArchivedSessions).not.toHaveBeenCalled();

  await user.click(within(dialog).getByTestId("confirm-delete-archived"));

  await waitFor(() => {
    expect(api.applyDeleteArchivedSessions).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      threadIds: [archivedThreadId]
    });
  });
  expect(screen.getByRole("article", { name: "活跃 provider 会话" })).toBeInTheDocument();
  expect(screen.queryByRole("article", { name: "归档 provider 会话" })).not.toBeInTheDocument();
  const completion = screen.getByRole("status");
  expect(completion).toHaveTextContent("已删除 1 个归档会话");
  expect(completion).toHaveTextContent(`${fixtureCodexHome}\\ai-session-migrator-backup-20260617-130000`);
  expect(screen.queryByText(archivedThreadId)).not.toBeInTheDocument();

  await user.click(within(completion).getByRole("button", { name: "复制备份路径" }));
  expect(desktopActions.copyText).toHaveBeenCalledWith(
    `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-130000`
  );
});

test("delete archived sessions shows a blocking loading dialog while writing files", async () => {
  const api = fakeApi();
  const pendingDelete = deferred<Awaited<ReturnType<MigrationApi["applyDeleteArchivedSessions"]>>>();
  vi.mocked(api.applyDeleteArchivedSessions).mockReturnValueOnce(pendingDelete.promise);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await screen.findByText("活跃 provider 会话");
  await user.selectOptions(screen.getByLabelText("来源 provider"), "__all__");
  await user.click(screen.getByTestId("delete-archived-button"));
  await user.click(within(screen.getByRole("dialog", { name: /delete archived sessions/i })).getByTestId("confirm-delete-archived"));

  await expectBlockingLoadingDialog("正在删除", "正在创建备份并删除已归档会话");

  pendingDelete.resolve({
    deletedThreads: [archivedThreadId],
    backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-130000`,
    dryRun: false
  });

  await waitFor(() => {
    expect(screen.queryByRole("dialog", { name: "正在删除" })).not.toBeInTheDocument();
  });
  expect(screen.getByRole("status")).toHaveTextContent("已删除 1 个归档会话");
});

test("successful migration can switch Codex provider and restart after confirmation", async () => {
  const api = fakeApi();
  vi.mocked(api.scanCodexHome)
    .mockResolvedValueOnce(scanResponse)
    .mockResolvedValueOnce(scanResponseAfterMigration);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await screen.findByText("活跃 provider 会话");
  await user.selectOptions(screen.getByLabelText("来源 provider"), "funai");
  await user.click(screen.getByRole("button", { name: /预览迁移/ }));
  await waitFor(() => {
    expect(api.previewProviderMigration).toHaveBeenCalled();
  });
  await user.click(screen.getByRole("button", { name: /确认迁移/ }));
  await user.click(within(screen.getByRole("dialog", { name: "确认迁移" })).getByRole("button", { name: "确认迁移" }));

  const dialog = await screen.findByRole("dialog", { name: "切换并重启 Codex" });
  expect(dialog).toHaveTextContent("目标 provider");
  expect(dialog).toHaveTextContent("yihubangg");
  expect(api.switchProviderAndRestart).not.toHaveBeenCalled();

  await user.click(within(dialog).getByRole("button", { name: "确认重启" }));

  await waitFor(() => {
    expect(api.switchProviderAndRestart).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      targetProvider: "yihubangg"
    });
  });
  expect(screen.getByRole("status")).toHaveTextContent("Codex 已按新 provider 配置重新启动。");
});

test("active session item can be archived after confirmation", async () => {
  const api = fakeApi();
  vi.mocked(api.scanCodexHome).mockResolvedValueOnce(scanResponse).mockResolvedValueOnce(scanResponseAfterArchive);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const activeRow = await screen.findByRole("article", { name: "活跃 provider 会话" });
  await user.click(within(activeRow).getByRole("button", { name: "归档" }));

  const dialog = screen.getByRole("dialog", { name: "确认归档会话" });
  expect(dialog).toHaveTextContent("将归档 1 个活跃会话");
  expect(api.applyArchiveSessions).not.toHaveBeenCalled();

  await user.click(within(dialog).getByRole("button", { name: "确认归档" }));

  await waitFor(() => {
    expect(api.applyArchiveSessions).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      threadIds: [activeThreadId]
    });
  });
  await waitFor(() => {
    expect(api.scanCodexHome).toHaveBeenCalledTimes(2);
  });
  const archivedRow = screen.getByRole("article", { name: "活跃 provider 会话" });
  expect(within(archivedRow).getByText("已归档")).toBeInTheDocument();
  expect(within(archivedRow).getByRole("button", { name: "激活" })).toBeInTheDocument();
  expect(screen.getByRole("status")).toHaveTextContent("已归档 1 个会话");
});

test("active session item shows a blocking loading dialog while archiving", async () => {
  const api = fakeApi();
  const pendingArchive = deferred<Awaited<ReturnType<MigrationApi["applyArchiveSessions"]>>>();
  vi.mocked(api.scanCodexHome).mockResolvedValueOnce(scanResponse).mockResolvedValueOnce(scanResponseAfterArchive);
  vi.mocked(api.applyArchiveSessions).mockReturnValueOnce(pendingArchive.promise);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  const activeRow = await screen.findByRole("article", { name: "活跃 provider 会话" });
  await user.click(within(activeRow).getByRole("button", { name: "归档" }));
  await user.click(within(screen.getByRole("dialog", { name: "确认归档会话" })).getByRole("button", { name: "确认归档" }));

  await expectBlockingLoadingDialog("正在归档", "正在创建备份并移动会话文件");

  pendingArchive.resolve({
    changedThreads: [activeThreadId],
    backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-150000`
  });

  await waitFor(() => {
    expect(screen.queryByRole("dialog", { name: "正在归档" })).not.toBeInTheDocument();
  });
  expect(await screen.findByRole("status")).toHaveTextContent("已归档 1 个会话");
});

test("archived session item can be activated after confirmation", async () => {
  const api = fakeApi();
  vi.mocked(api.scanCodexHome).mockResolvedValueOnce(scanResponse).mockResolvedValueOnce(scanResponseAfterActivate);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await user.selectOptions(screen.getByLabelText("来源 provider"), "__all__");
  const archivedRow = await screen.findByRole("article", { name: "归档 provider 会话" });
  await user.click(within(archivedRow).getByRole("button", { name: "激活" }));

  const dialog = screen.getByRole("dialog", { name: "确认激活会话" });
  expect(dialog).toHaveTextContent("将激活 1 个已归档会话");
  expect(api.applyActivateSessions).not.toHaveBeenCalled();

  await user.click(within(dialog).getByRole("button", { name: "确认激活" }));

  await waitFor(() => {
    expect(api.applyActivateSessions).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      threadIds: [archivedThreadId]
    });
  });
  await waitFor(() => {
    expect(api.scanCodexHome).toHaveBeenCalledTimes(2);
  });
  const activatedRow = screen.getByRole("article", { name: "归档 provider 会话" });
  expect(within(activatedRow).getByText("活跃")).toBeInTheDocument();
  expect(within(activatedRow).getByRole("button", { name: "归档" })).toBeInTheDocument();
  expect(screen.getByRole("status")).toHaveTextContent("已激活 1 个会话");
});

test("archived session item shows a blocking loading dialog while activating", async () => {
  const api = fakeApi();
  const pendingActivate = deferred<Awaited<ReturnType<MigrationApi["applyActivateSessions"]>>>();
  vi.mocked(api.scanCodexHome).mockResolvedValueOnce(scanResponse).mockResolvedValueOnce(scanResponseAfterActivate);
  vi.mocked(api.applyActivateSessions).mockReturnValueOnce(pendingActivate.promise);
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await user.selectOptions(screen.getByLabelText("来源 provider"), "__all__");
  const archivedRow = await screen.findByRole("article", { name: "归档 provider 会话" });
  await user.click(within(archivedRow).getByRole("button", { name: "激活" }));
  await user.click(within(screen.getByRole("dialog", { name: "确认激活会话" })).getByRole("button", { name: "确认激活" }));

  await expectBlockingLoadingDialog("正在激活", "正在创建备份并恢复会话文件");

  pendingActivate.resolve({
    changedThreads: [archivedThreadId],
    backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-160000`
  });

  await waitFor(() => {
    expect(screen.queryByRole("dialog", { name: "正在激活" })).not.toBeInTheDocument();
  });
  expect(await screen.findByRole("status")).toHaveTextContent("已激活 1 个会话");
});

test("archived session item can be deleted without selecting every archived row", async () => {
  const api = fakeApi();
  const { user } = await renderWorkflow(api);

  await user.click(screen.getByRole("button", { name: /扫描会话/ }));
  await user.selectOptions(screen.getByLabelText("来源 provider"), "__all__");
  const archivedRow = await screen.findByRole("article", { name: "归档 provider 会话" });
  await user.click(within(archivedRow).getByRole("button", { name: "删除" }));

  const dialog = screen.getByRole("dialog", { name: /delete archived sessions/i });
  expect(dialog).toHaveTextContent("将删除 1 个已归档会话");

  await user.click(within(dialog).getByTestId("confirm-delete-archived"));

  await waitFor(() => {
    expect(api.applyDeleteArchivedSessions).toHaveBeenCalledWith({
      codexHome: fixtureCodexHome,
      threadIds: [archivedThreadId]
    });
  });
  expect(screen.queryByRole("article", { name: "归档 provider 会话" })).not.toBeInTheDocument();
});
