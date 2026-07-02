export type ProviderOptionKind = "config" | "discovered";
export type ThreadLifecycle = "active" | "archived";

export type ThreadRow = {
  threadId: string;
  shortId: string;
  displayName: string;
  projectName: string | null;
  projectPath: string | null;
  path: string;
  fileProvider: string | null;
  configProvider: string | null;
  lifecycle: ThreadLifecycle;
  issueCodes: string[];
  severity: number;
  canMigrate: boolean;
  suggestedActionCode: string;
  suggestedActionValues: Record<string, string>;
  updatedAtMs: number;
};

export type DashboardModel = {
  codexHome: string;
  totalThreads: number;
  problemThreads: number;
  issueCounts: Record<string, number>;
  rows: ThreadRow[];
};

export type ProviderOption = {
  value: string;
  label: string;
  kind: ProviderOptionKind;
  recommended: boolean;
};

export type ProviderOptions = {
  currentConfigProvider: string | null;
  sourceProviders: string[];
  targetProviders: ProviderOption[];
};

export type ScanResponse = {
  dashboard: DashboardModel;
  providerOptions: ProviderOptions;
  configProvider: string | null;
};

export type PlannedRepair = {
  threadId: string;
  code: string;
  message: string;
};

export type MigrationRequest = {
  codexHome: string;
  sourceProvider: string | null;
  targetProvider: string;
  threadIds: string[];
};

export type DeleteArchivedRequest = {
  codexHome: string;
  threadIds: string[];
};

export type ArchiveRequest = {
  codexHome: string;
  threadIds: string[];
};

export type ProviderRestartRequest = {
  codexHome: string;
  targetProvider: string;
};

export type SessionTranscriptRequest = {
  codexHome: string;
  threadId: string;
  path: string;
};

export type ArchiveResult = {
  changedThreads: string[];
  backupDir: string | null;
};

export type DeleteArchivedResult = {
  deletedThreads: string[];
  backupDir: string | null;
  dryRun: boolean;
};

export type MigrationResult = {
  changedThreads: string[];
  plannedRepairs: PlannedRepair[];
  backupDir: string | null;
  dryRun: boolean;
};

export type ProviderRestartResult = {
  configuredProvider: string;
  previousProvider: string | null;
  configBackupDir: string | null;
  restartAttempted: boolean;
  restarted: boolean;
  restartMessage: string;
};

export type TranscriptRole = "user" | "assistant" | "system" | "tool" | "other";

export type TranscriptTurn = {
  role: TranscriptRole;
  text: string;
  timestamp: string | null;
  index: number;
};

export type SessionTranscript = {
  threadId: string;
  title: string;
  path: string;
  omittedTurns: number;
  turns: TranscriptTurn[];
};

export type CommandError = {
  code: string;
  message: string;
  backupDir?: string | null;
  operation?: string | null;
};
