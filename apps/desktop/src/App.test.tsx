import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, test, vi } from "vitest";
import App from "./App";
import type { MigrationApi } from "./domain/migrationApi";
import type { ScanResponse } from "./domain/session";

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

function fakeApi(): MigrationApi {
  return {
    scanCodexHome: vi.fn().mockResolvedValue(scanResponse),
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
    applyProviderMigration: vi.fn().mockResolvedValue({
      changedThreads: [activeThreadId],
      plannedRepairs: [],
      backupDir: `${fixtureCodexHome}\\ai-session-migrator-backup-20260617-120000`,
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
    })
  };
}

async function renderWorkflow(api = fakeApi()) {
  const user = userEvent.setup();
  render(<App migrationApi={api} resolveDefaultCodexHome={() => Promise.resolve(fixtureCodexHome)} />);
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
    />
  );
  await screen.findByDisplayValue(fixtureCodexHome);
  return { api, desktopActions, user };
}

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

  expect(within(activeRow).getByText("活跃")).toBeInTheDocument();
  expect(within(archivedRow).getByText("已归档")).toBeInTheDocument();
  expect(activeRow.compareDocumentPosition(archivedRow) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(screen.getByText("2 个可见，2 个已选")).toBeInTheDocument();
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
