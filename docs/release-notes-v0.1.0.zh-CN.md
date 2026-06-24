# AI Session Migrator v0.1.0

首个公开桌面版本。

## 它能做什么

AI Session Migrator 是一个本地优先的 Windows 和 macOS 桌面应用，用来把 Codex 会话从一个 AI provider 迁移到另一个 provider。

当你切换 provider 后，如果旧会话仍然绑定在之前的 provider 上，可以用它扫描本地会话、选择来源和目标 provider、预览迁移结果，并在写入前自动创建备份。

## 本版本包含

- 扫描指定 `.codex` 目录下的本地 Codex 会话。
- 识别活跃会话和已归档会话。
- 活跃会话优先展示，归档会话明确标识。
- 按来源 provider 筛选会话。
- 从下拉列表选择目标 provider，或输入自定义 provider。
- 写入前预览迁移计划。
- 真正迁移前弹窗确认。
- 迁移前自动创建备份。
- 确认和备份后删除选中的已归档会话。
- 操作完成后复制备份路径或打开备份目录。

## 下载

下载文件：

```text
AI-Session-Migrator-Windows-x64.exe
AI-Session-Migrator-macOS-universal-unsigned.dmg
```

## 注意事项

- macOS DMG 当前为未签名预览版。如果首次打开被系统阻止，可以在 Finder 中右键应用并选择 **打开**，或在 **系统设置 > 隐私与安全性** 中允许打开。
- 当前界面为中文。
- 应用只在本机处理数据，不上传会话文件。
- 早期未签名版本可能触发 Windows SmartScreen 提醒。

## 验证

本版本经过以下命令验证：

```powershell
npm test
npm --workspace apps/desktop run build
cd apps/desktop/src-tauri
cargo test --lib
```
