import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, test, vi } from "vitest";
import { resolveDesktopCodexHome } from "./defaultCodexHome";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn()
}));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});

test("resolves the default Codex home through the Tauri command", async () => {
  vi.mocked(invoke).mockResolvedValue(String.raw`C:\Users\jianrui\.codex`);

  await expect(resolveDesktopCodexHome()).resolves.toBe(String.raw`C:\Users\jianrui\.codex`);
  expect(invoke).toHaveBeenCalledWith("default_codex_home");
});
