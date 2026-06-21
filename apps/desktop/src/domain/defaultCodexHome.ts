import { invoke } from "@tauri-apps/api/core";

export async function resolveDesktopCodexHome() {
  try {
    const codexHome = await invoke<string>("default_codex_home");
    if (codexHome.trim().length > 0) {
      return codexHome;
    }
  } catch {
    // Browser dev mode and tests may not expose Tauri commands.
  }

  return fallbackCodexHome();
}

export function fallbackCodexHome() {
  const environment = (globalThis as typeof globalThis & {
    process?: { env?: Record<string, string | undefined> };
  }).process?.env;
  const home =
    environment?.USERPROFILE ??
    (environment?.HOMEDRIVE && environment?.HOMEPATH
      ? `${environment.HOMEDRIVE}${environment.HOMEPATH}`
      : environment?.HOME);

  if (!home) {
    return ".codex";
  }

  const normalizedHome = home.replace(/[\\/]+$/, "");
  const separator = normalizedHome.includes("\\") ? "\\" : "/";
  return `${normalizedHome}${separator}.codex`;
}
