// DiffViewer: Monaco diff editor for a single proposal.
//
// The component takes a DiffProposal and reads the current on-disk file
// content for the LEFT (original) side. The RIGHT (modified) side is
// proposal.new_content. If new_content is null (legacy pre-v2 row) we
// show the unified patch in a fallback pane and disable Approve.
//
// We deliberately read the on-disk content rather than reconstructing it
// from the patch because:
//   - SCC was already the truth source when the agent computed the diff
//   - the renderer has direct access to fs_read_file
//   - it correctly handles the "file changed between propose and resolve"
//     case (the diff will look wrong, prompting the user to re-prompt
//     the agent — proper 3-way merge is out of v1 scope)

import { invoke } from "@tauri-apps/api/core";
import { DiffEditor, type DiffOnMount } from "@monaco-editor/react";
import { useEffect, useState } from "react";

import type { DiffAction, DiffProposal } from "./types";

type Props = {
  proposal: DiffProposal;
  onResolved: (action: DiffAction) => void | Promise<void>;
};

// Map a file extension to a Monaco language id. Conservative list; Monaco
// falls back to plaintext for anything unmapped, which is fine.
function languageFor(path: string): string {
  const dot = path.lastIndexOf(".");
  if (dot < 0) return "plaintext";
  const ext = path.slice(dot + 1).toLowerCase();
  switch (ext) {
    case "ts":
    case "tsx":
      return "typescript";
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return "javascript";
    case "rs":
      return "rust";
    case "py":
      return "python";
    case "go":
      return "go";
    case "json":
      return "json";
    case "md":
      return "markdown";
    case "css":
      return "css";
    case "html":
      return "html";
    case "yaml":
    case "yml":
      return "yaml";
    case "toml":
      return "ini";
    case "sh":
    case "bash":
      return "shell";
    default:
      return "plaintext";
  }
}

export function DiffViewer({ proposal, onResolved }: Props) {
  const [original, setOriginal] = useState<string>("");
  const [originalLoading, setOriginalLoading] = useState<boolean>(true);
  const [originalError, setOriginalError] = useState<string | null>(null);
  const [busy, setBusy] = useState<boolean>(false);

  // Pull the current on-disk content for the original side. If the file
  // does not exist (a brand-new file proposal) we treat the original
  // side as empty.
  useEffect(() => {
    let cancelled = false;
    setOriginalLoading(true);
    setOriginalError(null);
    void invoke<string>("fs_read_file", { path: proposal.file_path })
      .then((content) => {
        if (cancelled) return;
        setOriginal(content);
      })
      .catch((e) => {
        if (cancelled) return;
        // Likely a new-file proposal. Empty-left is the right rendering.
        setOriginal("");
        setOriginalError(String(e));
      })
      .finally(() => {
        if (cancelled) return;
        setOriginalLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [proposal.file_path]);

  const newContent = proposal.new_content;
  const canApprove = newContent !== null;
  const language = languageFor(proposal.file_path);

  const handleResolve = async (action: DiffAction) => {
    if (busy) return;
    setBusy(true);
    try {
      await onResolved(action);
    } finally {
      setBusy(false);
    }
  };

  const onMount: DiffOnMount = (editor) => {
    editor.updateOptions({ readOnly: true });
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <div className="flex items-center justify-between gap-2 border-b border-border px-2 py-1">
        <div className="flex min-w-0 flex-col">
          <span className="truncate font-mono text-xs">
            {proposal.file_path}
          </span>
          <span className="text-[10px] text-muted-foreground">
            proposal #{proposal.id} session #{proposal.session_id}
            {originalError ? " (file not found; treating as new file)" : ""}
          </span>
        </div>
        <div className="flex shrink-0 gap-1">
          <button
            type="button"
            disabled={busy}
            onClick={() => void handleResolve("reject")}
            className="rounded border border-border px-2 py-0.5 text-xs disabled:opacity-50"
          >
            Reject
          </button>
          <button
            type="button"
            disabled={busy || !canApprove}
            title={canApprove ? "" : "Legacy proposal: new_content missing"}
            onClick={() => void handleResolve("approve")}
            className="rounded border border-border bg-primary px-2 py-0.5 text-xs text-primary-foreground disabled:opacity-50"
          >
            {busy ? "..." : "Approve"}
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1">
        {originalLoading ? (
          <div className="p-2 text-xs italic text-muted-foreground">
            loading original...
          </div>
        ) : newContent === null ? (
          // Legacy proposal: render the unified patch as a read-only pane.
          // Approve is disabled above.
          <pre className="h-full overflow-auto whitespace-pre-wrap p-2 font-mono text-xs">
            {proposal.patch}
          </pre>
        ) : (
          <DiffEditor
            height="100%"
            language={language}
            original={original}
            modified={newContent}
            theme="vs-dark"
            options={{
              readOnly: true,
              renderSideBySide: true,
              automaticLayout: true,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
            }}
            onMount={onMount}
          />
        )}
      </div>
    </div>
  );
}
