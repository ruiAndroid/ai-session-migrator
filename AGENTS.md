# AGENTS.md

## Project

- Name: ai-session-migrator
- Date: 2026-06-26
- Workspace root: `D:\dev\AI\AIPro\fun-claw\ai-session-migrator`

## Working Rules

- Default language: Chinese for discussion; code, commands, and identifiers stay in English.
- Start from first principles: explain why a decision matters and its impact on users.
- Before code or file work, read `AGENTS.md` and `MEMORY.md`.
- Keep `MEMORY.md` updated with durable architecture decisions, pitfalls, user corrections, and external resource locations.
- Do not store credential values in memory; store only where credentials live.

## Inferred Stack

- Desktop app: Tauri 2, Rust, React, TypeScript, Vite.
- Main desktop package: `apps/desktop`.
- Windows release artifacts are produced by GitHub Actions under `.github/workflows`.

