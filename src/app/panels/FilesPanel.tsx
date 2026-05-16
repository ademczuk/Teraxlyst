// FilesPanel: thin wrapper that forwards every FileExplorer prop. We
// keep FileExplorer's existing API surface untouched so the existing
// behaviour (open file, rename, reveal-in-terminal, attach-to-agent)
// keeps working when it is hosted inside SidebarTabs.

import { FileExplorer } from "@/modules/explorer";

type Props = {
  rootPath: string | null;
  onOpenFile: (path: string, pin?: boolean) => void;
  onPathRenamed?: (from: string, to: string) => void;
  onPathDeleted?: (path: string) => void;
  onRevealInTerminal?: (path: string) => void;
  onAttachToAgent?: (path: string) => void;
};

export default function FilesPanel(props: Props) {
  return (
    <div className="h-full bg-card">
      <FileExplorer {...props} />
    </div>
  );
}
