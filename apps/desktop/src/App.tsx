import {
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  FolderOpen,
  HardDrive,
  RefreshCw,
  ShieldCheck
} from "lucide-react";
import { useMemo, useState } from "react";
import { mockSessions } from "./domain/session";
import "./styles.css";

export default function App() {
  const [sourceProvider, setSourceProvider] = useState("");
  const [targetProvider, setTargetProvider] = useState("yihubangg");
  const [selectedIds, setSelectedIds] = useState<string[]>(mockSessions.map((session) => session.id));
  const [expandedIds, setExpandedIds] = useState<string[]>([]);

  const visibleSessions = useMemo(() => {
    const keyword = sourceProvider.trim().toLowerCase();
    return mockSessions.filter((session) => !keyword || session.sourceProvider.toLowerCase().includes(keyword));
  }, [sourceProvider]);

  const selectedVisibleCount = visibleSessions.filter((session) => selectedIds.includes(session.id)).length;
  const selectedCount = selectedIds.length;

  function toggleSession(id: string) {
    setSelectedIds((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id]
    );
  }

  function toggleDetails(id: string) {
    setExpandedIds((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id]
    );
  }

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="迁移设置">
        <div className="brand-block">
          <p className="eyebrow">AI Session Migrator</p>
          <h1>会话迁移助手</h1>
          <p className="muted">把本地会话从旧 provider 迁移到当前 provider，不上传任何数据。</p>
        </div>

        <section className="setup-card" aria-label="迁移步骤">
          <div className="step-row">
            <span className="step-number">1</span>
            <div>
              <strong>选择本地 Codex 目录</strong>
              <p>默认读取当前用户目录，也可以稍后改成手动选择。</p>
            </div>
          </div>

          <label className="field-label">
            Codex 目录
            <div className="path-input">
              <FolderOpen aria-hidden="true" size={17} />
              <input value="C:\\Users\\jianrui\\.codex" readOnly />
            </div>
          </label>

          <div className="step-row">
            <span className="step-number">2</span>
            <div>
              <strong>选择迁移方向</strong>
              <p>不知道旧 provider 名称时，可以先留空扫描。</p>
            </div>
          </div>

          <label className="field-label">
            从哪个 provider 迁出
            <input
              placeholder="留空自动识别"
              value={sourceProvider}
              onChange={(event) => setSourceProvider(event.target.value)}
            />
          </label>

          <label className="field-label">
            迁移到哪个 provider
            <input value={targetProvider} onChange={(event) => setTargetProvider(event.target.value)} />
          </label>

          <button className="primary-button" type="button">
            <RefreshCw aria-hidden="true" size={17} />
            扫描可迁移会话
          </button>
        </section>

        <section className="local-note" aria-label="本地安全说明">
          <HardDrive aria-hidden="true" size={18} />
          <div>
            <strong>本地处理</strong>
            <span>扫描、预览、备份和写入都会在你的电脑上完成。</span>
          </div>
        </section>
      </aside>

      <section className="workspace">
        <header className="summary">
          <div>
            <p className="eyebrow">Preview first</p>
            <h2>准备迁移 {selectedCount} 个会话</h2>
            <p className="muted">默认只预览。确认迁移前会自动创建备份，所有数据都留在本机。</p>
          </div>
          <div className="summary-actions">
            <button className="secondary-button" type="button">预览迁移</button>
            <button className="primary-button" type="button">
              <ShieldCheck aria-hidden="true" size={17} />
              确认迁移
            </button>
          </div>
        </header>

        <section className="steps" aria-label="迁移会做什么">
          <div>
            <CheckCircle2 aria-hidden="true" size={18} />
            <span>更新 provider 标记</span>
          </div>
          <div>
            <CheckCircle2 aria-hidden="true" size={18} />
            <span>修复会话列表索引</span>
          </div>
          <div>
            <CheckCircle2 aria-hidden="true" size={18} />
            <span>写入前自动备份</span>
          </div>
        </section>

        <section className="session-panel">
          <div className="panel-heading">
            <div>
              <h3>选择要迁移的会话</h3>
              <p className="muted">优先显示标题、时间和迁移建议；UUID、文件路径等高级信息默认收起。</p>
            </div>
            <span className="pill">{visibleSessions.length} 个可见，{selectedVisibleCount} 个已选</span>
          </div>

          <div className="session-list">
            {visibleSessions.map((session) => {
              const isExpanded = expandedIds.includes(session.id);
              const isSelected = selectedIds.includes(session.id);

              return (
                <article className="session-row" key={session.id} aria-label={session.title}>
                  <label className="session-select">
                    <input
                      aria-label={`选择会话：${session.title}`}
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => toggleSession(session.id)}
                    />
                    <span />
                  </label>

                  <div className="session-main">
                    <div className="session-title-line">
                      <strong>{session.title}</strong>
                      <span className={session.status === "ready" ? "status ready" : "status attention"}>
                        {session.status === "ready" ? "可迁移" : "需预览"}
                      </span>
                    </div>
                    <p>{session.reason}</p>
                    {isExpanded ? (
                      <div className="advanced-details">
                        <span>ID: {session.id}</span>
                        <span>Provider: {session.sourceProvider} -&gt; {targetProvider}</span>
                        <span>文件: {session.path}</span>
                      </div>
                    ) : null}
                  </div>

                  <div className="session-meta">
                    <span>{session.messageCount} 条消息</span>
                    <small>{session.updatedAt}</small>
                    <button
                      className="ghost-button"
                      type="button"
                      aria-expanded={isExpanded}
                      onClick={() => toggleDetails(session.id)}
                    >
                      {isExpanded ? <ChevronDown aria-hidden="true" size={16} /> : <ChevronRight aria-hidden="true" size={16} />}
                      查看高级信息
                    </button>
                  </div>
                </article>
              );
            })}
          </div>
        </section>
      </section>
    </main>
  );
}
