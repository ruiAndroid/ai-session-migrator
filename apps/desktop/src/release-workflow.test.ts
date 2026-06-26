import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const currentDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(currentDir, "..", "..", "..");
const releaseWindowsWorkflow = readFileSync(
  join(repoRoot, ".github", "workflows", "release-windows.yml"),
  "utf8"
);

describe("Windows release workflow", () => {
  it("publishes the bundled installer instead of the raw dev-server-backed executable", () => {
    expect(releaseWindowsWorkflow).toContain("desktop:bundle");
    expect(releaseWindowsWorkflow).toContain("bundle/nsis");
    expect(releaseWindowsWorkflow).toContain("AI-Session-Migrator-Windows-x64-setup.exe");
    expect(releaseWindowsWorkflow).not.toContain(
      "target/release/ai-session-migrator.exe"
    );
    expect(releaseWindowsWorkflow).not.toContain(
      "AI-Session-Migrator-Windows-x64.exe"
    );
  });
});

const tauriConfig = readFileSync(
  join(repoRoot, "apps", "desktop", "src-tauri", "tauri.conf.json"),
  "utf8"
);

describe("Tauri bundle config", () => {
  it("builds the Windows release installer without requiring the WiX MSI toolchain", () => {
    const config = JSON.parse(tauriConfig);

    expect(config.bundle.targets).toEqual(["nsis"]);
  });
});
