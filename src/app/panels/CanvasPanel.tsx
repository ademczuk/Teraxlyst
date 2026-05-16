// CanvasPanel: Excalidraw whiteboard mounted as a sidebar tab.
//
// Excalidraw is React-based but lazy-imported so the initial bundle
// stays small (the package is ~1.2 MB minified / ~350 KB gzipped).
// While the dynamic import resolves we show a thin loading placeholder.
//
// Font asset path is set in main.tsx via window.EXCALIDRAW_ASSET_PATH
// so Excalidraw self-hosts fonts from /public instead of fetching them
// from esm.run (which the current Tauri CSP `connect-src` does not
// allow without a wildcard).
//
// Persistence: not wired yet. Drawings live in component state and
// are lost on tab switch. A follow-up will reuse the M4 trackers
// schema or extend the DB with a `drawings` table.

import { lazy, Suspense } from "react";

const ExcalidrawLazy = lazy(async () => {
  const mod = await import("@excalidraw/excalidraw");
  await import("@excalidraw/excalidraw/index.css");
  return { default: mod.Excalidraw };
});

export default function CanvasPanel() {
  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="canvas-panel">
      <div className="flex items-center justify-between border-b border-border/60 bg-card px-3 py-2 text-xs">
        <span className="font-medium text-foreground">Canvas</span>
        <span className="text-muted-foreground">
          Excalidraw - drawings not persisted yet
        </span>
      </div>
      <div
        className="relative min-h-0 flex-1"
        data-testid="canvas-mount"
      >
        <Suspense
          fallback={
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
              loading canvas...
            </div>
          }
        >
          <ExcalidrawLazy
            theme="dark"
            initialData={{
              appState: {
                viewBackgroundColor: "#0f1115",
              },
            }}
          />
        </Suspense>
      </div>
    </div>
  );
}
