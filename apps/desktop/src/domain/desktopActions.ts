import { invoke } from "@tauri-apps/api/core";

export type DesktopActions = {
  openPath(path: string): Promise<void>;
  copyText(text: string): Promise<void>;
};

export const tauriDesktopActions: DesktopActions = {
  openPath(path) {
    return invoke<void>("open_path", { path });
  },
  async copyText(text) {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }

    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.top = "-1000px";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    const copied = document.execCommand("copy");
    document.body.removeChild(textarea);
    if (!copied) {
      throw new Error("clipboard unavailable");
    }
  }
};
