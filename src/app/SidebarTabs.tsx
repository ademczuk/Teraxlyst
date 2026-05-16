// SidebarTabs: vertical icon strip on the left edge + active sidebar
// panel to its right. Replaces the floating Sessions / Diffs overlay
// toggles previously added in M2.0 / M5.1.
//
// Tab state lives in this component (useState); sidebar contents are
// strictly local UI state and there is no need to plumb it through the
// global tabs store. Default tab is "files" so the existing
// FileExplorer-first behaviour is preserved on first launch.
//
// Each panel renders inside the same flex column so the
// ResizablePanel outside can size the whole sidebar uniformly.

import { cn } from "@/lib/utils";
import {
  BotIcon,
  CanvasIcon,
  CheckListIcon,
  FileDiffIcon,
  FlowchartIcon,
  Folder01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";

import CanvasPanel from "./panels/CanvasPanel";
import DiagramPanel from "./panels/DiagramPanel";
import DiffsPanel from "./panels/DiffsPanel";
import FilesPanel from "./panels/FilesPanel";
import SessionsPanel from "./panels/SessionsPanel";
import TrackersPanel from "./panels/TrackersPanel";

type TabId =
  | "files"
  | "sessions"
  | "trackers"
  | "diffs"
  | "diagram"
  | "canvas";

type TabSpec = {
  id: TabId;
  label: string;
  icon: typeof Folder01Icon;
};

const TABS: TabSpec[] = [
  { id: "files", label: "Files", icon: Folder01Icon },
  { id: "sessions", label: "Sessions", icon: BotIcon },
  { id: "trackers", label: "Trackers", icon: CheckListIcon },
  { id: "diffs", label: "Diffs", icon: FileDiffIcon },
  { id: "diagram", label: "Diagram", icon: FlowchartIcon },
  { id: "canvas", label: "Canvas", icon: CanvasIcon },
];

type FilesProps = {
  rootPath: string | null;
  onOpenFile: (path: string, pin?: boolean) => void;
  onPathRenamed?: (from: string, to: string) => void;
  onPathDeleted?: (path: string) => void;
  onRevealInTerminal?: (path: string) => void;
  onAttachToAgent?: (path: string) => void;
};

type Props = {
  files: FilesProps;
  onActiveChange?: (id: TabId) => void;
};

// Tabs that need a wide canvas (Mermaid SVG, Excalidraw whiteboard)
// rather than a narrow nav strip. App.tsx watches the active tab via
// onActiveChange and resizes the sidebar panel accordingly.
export const WIDE_TABS: ReadonlySet<TabId> = new Set([
  "diagram",
  "canvas",
]);

export function SidebarTabs({ files, onActiveChange }: Props) {
  const [active, setActive] = useState<TabId>("files");

  const handlePick = (id: TabId) => {
    setActive(id);
    onActiveChange?.(id);
  };

  return (
    <div className="flex h-full min-h-0">
      <div
        role="tablist"
        aria-orientation="vertical"
        className="flex w-10 shrink-0 flex-col items-center gap-1 border-r border-border/60 bg-card py-2"
      >
        {TABS.map((t) => {
          const isActive = t.id === active;
          return (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={isActive}
              title={t.label}
              onClick={() => handlePick(t.id)}
              className={cn(
                "relative flex h-8 w-8 items-center justify-center rounded text-muted-foreground transition-colors",
                isActive
                  ? "bg-muted/60 text-foreground"
                  : "hover:bg-muted/40 hover:text-foreground",
              )}
            >
              {isActive ? (
                <span
                  aria-hidden
                  className="absolute left-0 top-1 h-6 w-[2px] rounded-r bg-primary"
                />
              ) : null}
              <HugeiconsIcon icon={t.icon} size={16} />
            </button>
          );
        })}
      </div>

      <div className="min-h-0 flex-1 overflow-hidden">
        {active === "files" ? <FilesPanel {...files} /> : null}
        {active === "sessions" ? <SessionsPanel /> : null}
        {active === "trackers" ? <TrackersPanel /> : null}
        {active === "diffs" ? <DiffsPanel /> : null}
        {active === "diagram" ? <DiagramPanel /> : null}
        {active === "canvas" ? <CanvasPanel /> : null}
      </div>
    </div>
  );
}
