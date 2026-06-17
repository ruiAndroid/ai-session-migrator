export type SessionStatus = "ready" | "needs_attention";

export type SessionRow = {
  id: string;
  shortId: string;
  title: string;
  sourceProvider: string;
  targetProvider: string;
  status: SessionStatus;
  reason: string;
  updatedAt: string;
  messageCount: number;
  path: string;
};

export const mockSessions: SessionRow[] = [
  {
    id: "019eca3b-941d-7340-9b14-328c635a6523",
    shortId: "019eca3b",
    title: "恢复 provider 切换后的会话",
    sourceProvider: "funai",
    targetProvider: "yihubangg",
    status: "ready",
    reason: "检测到旧 provider 标记，可以迁移到当前 provider。",
    updatedAt: "2026-06-15 19:12",
    messageCount: 42,
    path: "C:\\Users\\jianrui\\.codex\\sessions\\019eca3b-941d-7340-9b14-328c635a6523.jsonl"
  },
  {
    id: "019ec94d-720d-7a12-a379-28c8042bc6b4",
    shortId: "019ec94d",
    title: "讨论会话迁移工具开源方向",
    sourceProvider: "funai",
    targetProvider: "yihubangg",
    status: "ready",
    reason: "会话内容完整，迁移前会自动创建备份。",
    updatedAt: "2026-06-15 18:43",
    messageCount: 28,
    path: "C:\\Users\\jianrui\\.codex\\sessions\\019ec94d-720d-7a12-a379-28c8042bc6b4.jsonl"
  },
  {
    id: "019ec3ee-c12d-7aa0-a19d-430aa4ee1979",
    shortId: "019ec3ee",
    title: "桌面版迁移助手产品设计",
    sourceProvider: "gmn",
    targetProvider: "yihubangg",
    status: "needs_attention",
    reason: "源 provider 与当前筛选不一致，建议预览后再迁移。",
    updatedAt: "2026-06-15 16:20",
    messageCount: 36,
    path: "C:\\Users\\jianrui\\.codex\\sessions\\019ec3ee-c12d-7aa0-a19d-430aa4ee1979.jsonl"
  }
];
