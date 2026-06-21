# 中文推广文案

## 一句话介绍

AI Session Migrator 是一个本地优先的桌面工具，用来把 Codex 会话从旧 AI provider 迁移到新 provider，支持预览、备份和归档会话清理。

## GitHub About

**Description**

```text
本地优先的 Codex 会话 provider 迁移桌面工具。
```

**Topics**

```text
codex
codex-desktop
ai-tools
session-migration
provider-migration
tauri
react
typescript
rust
desktop-app
windows
local-first
```

## 短推广文案

我做了一个小工具：AI Session Migrator。

它是给 Codex Desktop 用户用的 provider 迁移桌面应用。可以扫描本机会话，选择来源 provider 和目标 provider，先预览，再备份，最后迁移；也可以清理已归档会话。

重点是本地优先：不遥测，不上传，不远程解析会话文件。

Windows 下载：

```text
https://github.com/ruiAndroid/ai-session-migrator/releases
```

## 长推广文案

AI Session Migrator 解决的是一个很具体的问题：切换 AI provider 之后，旧的 Codex 会话可能还绑定在之前的 `model_provider` 上。

这个工具提供了一个桌面版流程：扫描本地 Codex 会话，区分活跃和已归档会话，显示每个 provider 的数量，选择来源 provider 和目标 provider，写入前预览，写入前备份，完成后可以复制或打开备份目录。

它也支持删除已归档会话，但会先确认并创建备份，避免误删。

会话文件里可能包含提示词、代码、本机路径和业务上下文，所以这个项目坚持本地处理：不上传、不遥测、不远程解析。

技术栈：Tauri、React、TypeScript、Rust。

GitHub：

```text
https://github.com/ruiAndroid/ai-session-migrator
```

## Release 页面建议

- 放一张主界面扫描结果截图。
- 放一张迁移预览截图。
- 放一张迁移完成后备份操作提示截图。
- 上传 `AI-Session-Migrator-Windows-x64.exe`。
- 说明早期未签名版本可能触发 Windows SmartScreen。
- 强调本地处理和写入前备份。
