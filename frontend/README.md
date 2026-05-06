# RustChat Frontend

Vue 3 + TypeScript + Vite SPA for RustChat.

## Commands

```bash
cd frontend
npm ci
npm run dev
npm run build
npm run test:unit
npm run check:dependency-policy
```

## Supply-Chain Rules

1. Use `npm` only for the frontend workspace.
2. Commit `package-lock.json` for every dependency change.
3. CI installs dependencies with `npm ci --ignore-scripts`.
4. New direct dependencies must be documented in `frontend/dependency-policy.json`.
5. See [../docs/frontend-dependency-policy.md](../docs/frontend-dependency-policy.md) before adding or changing dependencies.
