// SessionsPanel: sidebar wrapper around the SessionList. Replaces the
// floating-toggle overlay added in M2.0. Layout matches the FileExplorer
// pane: card background, right-side border supplied by SidebarTabs.
//
// SessionList already exposes its own "New session" prompt + Start button,
// so we don't duplicate that here. The header is kept minimal and just
// confirms which panel is currently active.

import { SessionList } from "@/modules/sessions";

export default function SessionsPanel() {
  return (
    <div className="flex h-full min-h-0 flex-col bg-card">
      <div className="border-b border-border/60 px-3 py-2">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Sessions
        </h2>
      </div>
      <div className="min-h-0 flex-1 overflow-hidden">
        <SessionList />
      </div>
    </div>
  );
}
