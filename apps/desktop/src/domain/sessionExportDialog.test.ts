import { beforeEach, expect, test, vi } from "vitest";
import {
  normalizeJsonlDestination,
  sessionExportFileName,
  tauriSessionExportDialog
} from "./sessionExportDialog";

const { saveMock } = vi.hoisted(() => ({
  saveMock: vi.fn()
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: saveMock
}));

beforeEach(() => {
  saveMock.mockReset();
});

test("opens Save As with the original JSONL filename", async () => {
  saveMock.mockResolvedValue("D:\\Exports\\rollout-a.jsonl");

  const result = await tauriSessionExportDialog.chooseDestination(
    "D:\\Codex\\.codex\\sessions\\rollout-a.jsonl"
  );

  expect(saveMock).toHaveBeenCalledWith({
    defaultPath: "rollout-a.jsonl",
    filters: [{ name: "Codex 会话", extensions: ["jsonl"] }]
  });
  expect(result).toBe("D:\\Exports\\rollout-a.jsonl");
});

test("extracts default filenames from Windows and Unix paths", () => {
  expect(sessionExportFileName("D:\\Codex\\sessions\\rollout-a.jsonl")).toBe(
    "rollout-a.jsonl"
  );
  expect(sessionExportFileName("/Users/rui/.codex/sessions/rollout-b.jsonl")).toBe(
    "rollout-b.jsonl"
  );
});

test("returns null when Save As is cancelled", async () => {
  saveMock.mockResolvedValue(null);

  await expect(
    tauriSessionExportDialog.chooseDestination("D:\\Codex\\sessions\\rollout-a.jsonl")
  ).resolves.toBeNull();
});

test("adds the JSONL extension when the destination omits it", () => {
  expect(normalizeJsonlDestination("D:\\Exports\\rollout-a")).toBe(
    "D:\\Exports\\rollout-a.jsonl"
  );
});

test("rejects a conflicting destination extension", () => {
  expect(() => normalizeJsonlDestination("D:\\Exports\\rollout-a.txt")).toThrow(
    "导出文件必须使用 .jsonl 后缀。"
  );
});
