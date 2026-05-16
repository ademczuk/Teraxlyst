// DiffsPanel: sidebar wrapper around DiffInbox. The DiffInbox component
// already renders a two-column list + DiffViewer layout, so we just
// embed it inside a card-styled sidebar container. The Monaco viewer
// renders inline (mirroring the editor stack pattern) rather than in a
// modal so the user keeps the list + viewer visible together.

import { DiffInbox } from "@/modules/diff";

export default function DiffsPanel() {
  return (
    <div className="flex h-full min-h-0 flex-col bg-card">
      <DiffInbox />
    </div>
  );
}
