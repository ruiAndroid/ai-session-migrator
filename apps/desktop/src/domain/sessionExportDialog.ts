import { save } from "@tauri-apps/plugin-dialog";

export type SessionExportDialog = {
  chooseDestination(sourcePath: string): Promise<string | null>;
};

export function sessionExportFileName(sourcePath: string) {
  const segments = sourcePath.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? "codex-session.jsonl";
}

export function normalizeJsonlDestination(path: string) {
  const fileName = sessionExportFileName(path);
  const lastDot = fileName.lastIndexOf(".");
  if (lastDot < 0) {
    return `${path}.jsonl`;
  }
  if (fileName.slice(lastDot).toLowerCase() === ".jsonl") {
    return path;
  }
  throw new Error("导出文件必须使用 .jsonl 后缀。");
}

export const tauriSessionExportDialog: SessionExportDialog = {
  async chooseDestination(sourcePath) {
    const destination = await save({
      defaultPath: sessionExportFileName(sourcePath),
      filters: [{ name: "Codex 会话", extensions: ["jsonl"] }]
    });
    return destination ? normalizeJsonlDestination(destination) : null;
  }
};
