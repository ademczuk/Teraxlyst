# Frontend Testing

Vitest is wired into the TypeScript side of Teraxlyst. The Rust side already
runs `cargo test --lib` on Linux CI; this brings the frontend up to parity for
the cheap-to-test pure-function layer.

## Running locally

```
pnpm install
pnpm test          # one-shot, used by CI
pnpm test:watch    # interactive watch mode
```

Tests run under the `jsdom` environment so `window.localStorage`,
`document`, and DOM APIs are available without a real browser. Globals
(`describe`, `it`, `expect`) are enabled via the vitest config so
individual test files don't need to import them.

Setup file `src/test-setup.ts` registers `@testing-library/jest-dom`
matchers so DOM assertions like `toBeInTheDocument()` are available once
component tests start landing.

## What's covered

- `src/lib/migration.test.ts`: the M3.1 one-time `terax-* -> teraxlyst-*`
  localStorage migration. Five cases covering happy path, no-overwrite of
  existing new keys, idempotence, exception swallowing, and the
  no-legacy-keys path.
- `src/modules/sessions/useSessionManager.test.ts`: the transcript ring
  buffer reducer `applyTranscriptBatch`. Four cases covering append,
  RING_CAP truncation, per-session isolation, and the `completed`-event
  side effect on `runningIds`. The reducer was extracted from the
  `useSessions` effect into a pure function so it could be tested without
  spinning up the Tauri event subscription.

Total: 9 tests across 2 files. Runs in under 3 seconds locally.

## What's intentionally not covered

- The `useSessions` hook itself. The effect body wires up
  `@tauri-apps/api/event#listen`, which has no jsdom shim. The pure
  reducer underneath is what holds the interesting invariants.
- Tracker field validation. Validation lives in Rust (`tracker_*`
  Tauri commands); there is no TS-side schema to assert against.
- MCP form-widget components in `src/modules/mcp/fields/`. Rendering
  tests would be useful eventually but the components are thin wrappers
  over radix primitives; the YAML-driven contract is enforced server-side.

## E2E

Real end-to-end coverage (Tauri window lifecycle, IPC, file dialogs)
needs `tauri-driver` + Playwright. That's a post-0.1.0 milestone
(tracked as M0.x); the headless harness needs Linux-arm64 wiring before
it's worth standing up.
