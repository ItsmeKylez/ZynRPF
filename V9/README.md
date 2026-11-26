# Zyn-RPF Web

Browser-based conversion worktree for the original Zyn-RPF FiveM asset editor.

## Current status

Milestone 1.4 adds RPF7 standard-file editing/repacking, raw RSC7 resource passthrough, and nested `.rpf` archive inspection. See `MILESTONE_1.4.md` for tested behavior and limitations.

## Local development

Requirements: Node.js 20+.

```bash
npm install
npm run dev
```

Build the static frontend:

```bash
npm run build
```

The build output is `dist/`.

## Deployment status

This worktree is **not yet production complete**. The original Rust engine is preserved under `reference/original-source/core`. A Rust API service using that engine, production upload/session boundaries, persistent storage, authentication, Railway deployment verification and complete original feature parity are still required before public deployment.

## Source preservation

The original GPL-3.0-only project is preserved under `reference/original-source/`. Keep its license and third-party notices with redistributed builds.
