# Docs Site Deployment (`docs.rustchat.io`)

This guide describes how to deploy the documentation site from GitHub to Cloudflare Pages using Cloudflare's native Git integration.

## Overview

- Source of truth: `docs/` directory in this repository
- Static site generator: VitePress
- CI quality checks: `.github/workflows/docs-ci-cd.yml`
- Deployment method: Cloudflare Pages Git integration (no GitHub deployment secrets)

## 1. Create a Cloudflare Pages Project

In Cloudflare Dashboard:

1. Go to `Workers & Pages`
2. Create a new `Pages` project
3. Connect it to your repository for automatic deploys from `main`
4. Note the project name (for example `rustchat-docs`)

## 2. Connect GitHub Repository

In the Pages project:

1. Open `Settings` -> `Build & deployments`
2. Connect the `kubedoio/rustchat` repository
3. Configure build:
   - Root directory: `docs`
   - Build command: `npm run docs:build`
   - Build output directory: `.vitepress/dist`
   - Deploy command (if required by your Cloudflare mode): `npx wrangler versions upload`

The repository includes `docs/wrangler.jsonc` so `wrangler versions upload` can resolve:
- Worker name (`docs-rustchat-io`)
- Compatibility date
- Assets directory (`.vitepress/dist`)

## 3. Workflow Behavior

- On pull requests: runs docs link checks and builds the VitePress site.
- On push to `main`: Cloudflare Pages deploys from the connected repository.
- GitHub Actions stays as an independent quality gate.

## 4. Configure Custom Domain

In the Pages project:

1. Open `Custom domains`
2. Add `docs.rustchat.io`
3. Complete DNS validation if required

Cloudflare manages TLS automatically once the domain is attached.

## 5. Manual Validation

```bash
cd docs
npm ci
npm run docs:check-links
npm run docs:build
```

If CI succeeds and Cloudflare Git integration is active, `docs.rustchat.io` updates automatically after merge to `main`.
