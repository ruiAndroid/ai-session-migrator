import { invoke } from "@tauri-apps/api/core";
import type {
  DeleteArchivedRequest,
  DeleteArchivedResult,
  MigrationRequest,
  MigrationResult,
  ScanResponse
} from "./session";

export type MigrationApi = {
  scanCodexHome(codexHome: string): Promise<ScanResponse>;
  previewProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
  applyProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
  previewDeleteArchivedSessions(request: DeleteArchivedRequest): Promise<DeleteArchivedResult>;
  applyDeleteArchivedSessions(request: DeleteArchivedRequest): Promise<DeleteArchivedResult>;
};

export const tauriMigrationApi: MigrationApi = {
  scanCodexHome(codexHome) {
    return invoke<ScanResponse>("scan_codex_home", { codexHome });
  },
  previewProviderMigration(request) {
    return invoke<MigrationResult>("preview_provider_migration", { request });
  },
  applyProviderMigration(request) {
    return invoke<MigrationResult>("apply_provider_migration", { request });
  },
  previewDeleteArchivedSessions(request) {
    return invoke<DeleteArchivedResult>("preview_delete_archived_sessions", { request });
  },
  applyDeleteArchivedSessions(request) {
    return invoke<DeleteArchivedResult>("apply_delete_archived_sessions", { request });
  }
};
