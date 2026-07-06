import { invoke } from "@tauri-apps/api/core";
import type {
  ArchiveRequest,
  ArchiveResult,
  CatalogRepairRequest,
  CatalogRepairResult,
  CatalogRepairScanResponse,
  DeleteArchivedRequest,
  DeleteArchivedResult,
  MigrationRequest,
  MigrationResult,
  ProviderRestartRequest,
  ProviderRestartResult,
  ScanResponse,
  SessionTranscript,
  SessionTranscriptRequest
} from "./session";

export type MigrationApi = {
  scanCodexHome(codexHome: string): Promise<ScanResponse>;
  scanCodexCatalogRepair(codexHome: string): Promise<CatalogRepairScanResponse>;
  previewProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
  previewCodexCatalogRepair(request: CatalogRepairRequest): Promise<CatalogRepairResult>;
  applyProviderMigration(request: MigrationRequest): Promise<MigrationResult>;
  applyCodexCatalogRepair(request: CatalogRepairRequest): Promise<CatalogRepairResult>;
  previewDeleteArchivedSessions(request: DeleteArchivedRequest): Promise<DeleteArchivedResult>;
  applyDeleteArchivedSessions(request: DeleteArchivedRequest): Promise<DeleteArchivedResult>;
  applyArchiveSessions(request: ArchiveRequest): Promise<ArchiveResult>;
  applyActivateSessions(request: ArchiveRequest): Promise<ArchiveResult>;
  switchProviderAndRestart(request: ProviderRestartRequest): Promise<ProviderRestartResult>;
  readSessionTranscript(request: SessionTranscriptRequest): Promise<SessionTranscript>;
};

export const tauriMigrationApi: MigrationApi = {
  scanCodexHome(codexHome) {
    return invoke<ScanResponse>("scan_codex_home", { codexHome });
  },
  scanCodexCatalogRepair(codexHome) {
    return invoke<CatalogRepairScanResponse>("scan_codex_catalog_repair", { codexHome });
  },
  previewProviderMigration(request) {
    return invoke<MigrationResult>("preview_provider_migration", { request });
  },
  previewCodexCatalogRepair(request) {
    return invoke<CatalogRepairResult>("preview_codex_catalog_repair", { request });
  },
  applyProviderMigration(request) {
    return invoke<MigrationResult>("apply_provider_migration", { request });
  },
  applyCodexCatalogRepair(request) {
    return invoke<CatalogRepairResult>("apply_codex_catalog_repair", { request });
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
  },
  readSessionTranscript(request) {
    return invoke<SessionTranscript>("read_session_transcript", { request });
  }
};
