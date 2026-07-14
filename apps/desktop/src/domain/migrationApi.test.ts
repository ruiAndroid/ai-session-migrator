import { beforeEach, expect, test, vi } from "vitest";
import { tauriMigrationApi } from "./migrationApi";
import type { SessionExportRequest } from "./session";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn()
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock
}));

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({
    threadId: "thread-a",
    destinationPath: "D:\\Exports\\rollout-a.jsonl",
    bytesWritten: 42
  });
});

test("exportSession invokes the raw session export command", async () => {
  const request: SessionExportRequest = {
    codexHome: "D:\\Codex\\.codex",
    threadId: "thread-a",
    sourcePath: "D:\\Codex\\.codex\\sessions\\rollout-a.jsonl",
    destinationPath: "D:\\Exports\\rollout-a.jsonl"
  };

  await tauriMigrationApi.exportSession(request);

  expect(invokeMock).toHaveBeenCalledWith("export_session", { request });
});
