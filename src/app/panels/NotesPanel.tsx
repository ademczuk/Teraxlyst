// NotesPanel: Lexical-backed WYSIWYG markdown editor.
//
// Markdown round-trips via @lexical/markdown TRANSFORMERS - typing
// `**foo**` becomes bold, `# heading` becomes h1, etc. The OnChange
// handler serializes the current editor state back to markdown and
// caches it in component state so we can re-render the source preview
// pane on the right.
//
// Persistence: not wired yet. Notes live in component state and are
// lost on tab switch. Follow-up will reuse the M4 trackers schema or
// extend the DB with a `notes` table.
//
// Sizes: full @lexical/* set is ~61 KB gzipped (lazy-loaded). The
// existing Teraxlyst CSP is sufficient - v0.44.0 specifically landed
// CSP-safe style updates (PR #8372).

import { lazy, Suspense } from "react";

const EditorLazy = lazy(async () => {
  const mod = await import("../../modules/notes/MarkdownEditor");
  return { default: mod.MarkdownEditor };
});

const SEED = `# Notes

A live WYSIWYG editor backed by **Lexical** 0.44.

Type \`# heading\`, \`**bold**\`, \`*italic*\`, \`\` \`code\` \`\`, \`- list item\`,
or paste a markdown link like [Teraxlyst](https://github.com/ademczuk/Teraxlyst)
and it round-trips automatically.

## Capabilities right now

- Headings, lists, links, inline code
- Undo/redo via the HistoryPlugin
- Markdown shortcuts (typed source becomes rich text)
- Round-trip serialization back to markdown source

## Not yet

- Persistence (notes are lost on tab switch)
- Code-block syntax highlighting
- Image embed
- Tracker-style cross-linking
`;

export default function NotesPanel() {
  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="notes-panel">
      <div className="flex items-center justify-between border-b border-border/60 bg-card px-3 py-2 text-xs">
        <span className="font-medium text-foreground">Notes</span>
        <span className="text-muted-foreground">
          Lexical WYSIWYG - not persisted yet
        </span>
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        <Suspense
          fallback={
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
              loading editor...
            </div>
          }
        >
          <EditorLazy initialMarkdown={SEED} />
        </Suspense>
      </div>
    </div>
  );
}
