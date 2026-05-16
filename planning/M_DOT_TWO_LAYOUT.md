# M.2 Layout: sidebar tabs replace floating overlays

## Why

M2.0 and M5.1 added Sessions and Diff Inbox as floating-toggle overlays
in the bottom-right of `App.tsx`. They shadowed the status bar, fought
the AI input bar for z-index, and could not coexist on screen with the
file tree. M.2 replaces them with a proper left sidebar that hosts
Files / Sessions / Trackers / Diffs behind an icon tab strip.

## Design

Left sidebar gains a vertical 40px icon strip on its left edge:

```
[Files]      Folder01Icon
[Sessions]   BotIcon
[Trackers]   CheckListIcon
[Diffs]      FileDiffIcon
```

Clicking an icon swaps the panel rendered to its right. The active tab
gets a 2px primary-coloured left border + muted background. The whole
sidebar still lives inside the existing `ResizablePanel` so its width
is user-controllable; default size bumped from 225px to 265px and
min/max widened so the panels fit comfortably.

State machine is just `useState<"files" | "sessions" | "trackers" | "diffs">`
inside `SidebarTabs`. Default is `"files"` so the FileExplorer-first
behaviour is preserved. No global store needed - the tab is pure local
UI state.

## Files added

- `src/app/SidebarTabs.tsx` - icon strip + active panel switcher.
- `src/app/panels/FilesPanel.tsx` - forwards every prop to FileExplorer
  unchanged. Keeps the rename / delete / reveal-in-terminal /
  attach-to-agent wiring identical to the old standalone mount.
- `src/app/panels/SessionsPanel.tsx` - wraps `SessionList` (which
  already ships its own "Start session" prompt).
- `src/app/panels/TrackersPanel.tsx` - list/table state machine. Calls
  `db_list_workspaces`, picks workspaces[0], invokes
  `tracker_load_workspace` to parse YAML + sync to DB, then renders the
  tracker list. Click drills into a TrackerTable that calls
  `tracker_list_items` and renders up to four schema fields in a
  compact table. Back arrow returns to the list. Parse errors surface
  inline under the list.
- `src/app/panels/DiffsPanel.tsx` - wraps `DiffInbox` (which already has
  list + Monaco viewer side-by-side, so no modal needed).

## Files modified

- `src/app/App.tsx`:
  - Removed `sessionsOpen` / `diffsOpen` state and both floating
    overlay JSX blocks.
  - Replaced direct `FileExplorer` mount with `<SidebarTabs files=...>`.
  - Dropped `FileExplorer`, `SessionList`, `DiffInbox` imports.
  - Widened the sidebar resizable bounds.

## FileExplorer behaviour

Kept unchanged. `FilesPanel` is a one-liner pass-through; the file tree
still receives `rootPath`, `onOpenFile`, `onPathRenamed`,
`onPathDeleted`, `onRevealInTerminal`, `onAttachToAgent` and routes
them up to `App` exactly as before. `PromptForUserInputDialog` stays
at the top level - it is a modal triggered by MCP events from
anywhere, not a panel.

## TS / build notes

- `pnpm exec tsc --noEmit` - clean (EXIT=0).
- `pnpm build` - clean (12.46s, EXIT=0).
- The Rust commands' camelCase IPC arg names (`workspaceId`,
  `workspacePath`, `args`) match how `SessionList` already invokes
  them, so no serde mismatch.
- TrackerItem rows are parsed with a try/catch around `JSON.parse` so
  a malformed `fields_json` blanks the cells rather than crashes the
  panel.

## Known visual issues

- The Trackers panel currently displays raw `fields_json` keys as
  column headers. A richer renderer (column ordering by role, type-
  aware cells, status pills, edit-in-place) is M4.x.
- DiffInbox's internal 180px list + viewer split assumes a wider host
  than the default 265px sidebar. Users can drag the resizable handle
  out to ~500px when reviewing diffs. A milestone-later fix would
  collapse the inner list when the host is narrow.
- No "+ New session" affordance is added on top of SessionList itself
  because SessionList already ships a prompt textarea + Start button.
