import "@testing-library/jest-dom/vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import SplashScreen from "./SplashScreen";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function mockReducedMotion(matches: boolean) {
  vi.stubGlobal(
    "matchMedia",
    vi.fn().mockImplementation((query: string) => ({
      matches: query.includes("prefers-reduced-motion") ? matches : false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn()
    }))
  );
}

test("startup splash introduces the desktop migrator brand and session flow", () => {
  mockReducedMotion(false);

  render(<SplashScreen durationMs={0} onComplete={vi.fn()} />);

  const splash = screen.getByRole("status", { name: "AI Session Migrator 启动闪屏" });
  expect(splash).toHaveTextContent("AI Session Migrator");
  expect(splash).toHaveTextContent("Codex 会话迁移助手");
  expect(splash).toHaveTextContent("provider");
  expect(screen.getAllByLabelText(/会话流节点/)).toHaveLength(5);
});

test("startup splash completes immediately when reduced motion is preferred", async () => {
  mockReducedMotion(true);
  const onComplete = vi.fn();

  render(<SplashScreen durationMs={1800} onComplete={onComplete} />);

  await waitFor(() => {
    expect(onComplete).toHaveBeenCalledTimes(1);
  });
});
