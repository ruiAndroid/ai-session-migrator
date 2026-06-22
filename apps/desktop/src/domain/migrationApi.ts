import { invoke } from "@tauri-apps/api/core";
import type {
  ArchiveRequest,
  ArchiveResult,
  DeleteArchivedRequest,
  DeleteArchivedResult,
  MigrationRequest,
  MigrationResult,
  ProviderRestartRequest,
  ProviderRestartResult,
  ScanResponse
} from "./session";

export type MigrationApi = {
  scanCodexHome(codexHome: string): Promise<ScanResponse>;
  previewProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
  applyProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
  previewDeleteArchivedSessions(request: DeleteArchivedRequest): Promise<DeleteArchivedResult>;
  applyDeleteArchivedSessions(request: DeleteArchivedRequest): Promise<DeleteArchivedResult>;
  applyArchiveSessions(request: ArchiveRequest): Promise<ArchiveResult>;
  applyActivateSessions(request: ArchiveRequest): Promise<ArchiveResult>;
  switchProviderAndRestart(request: ProviderRestartRequest): Promise<ProviderRestartResult>;
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
  },
  applyArchiveSessions(request) {
    return invoke<ArchiveResult>("apply_archive_sessions", { request });
  },
  applyActivateSessions(request) {
    return invoke<ArchiveResult>("apply_activate_sessions", { request });
  },
  switchProviderAndRestart(request) {
    return invoke<ProviderRestartResult>("switch_provider_and_restart", { request });
  }
};
