// MarkdownEditor: thin wrapper around Lexical's RichTextPlugin that
// supports markdown round-trip via @lexical/markdown TRANSFORMERS.
//
// Public API:
//   <MarkdownEditor
//     initialMarkdown="# hello"
//     onChange={(md) => setSource(md)}
//   />
//
// Wraps the editor in a flex column so the ContentEditable fills its
// parent height. The MarkdownInit child runs once on mount via a
// useEffect to seed the editor without retriggering on every render.

import "./markdown-editor.css";

import { CodeNode } from "@lexical/code";
import { LinkNode } from "@lexical/link";
import { ListItemNode, ListNode } from "@lexical/list";
import {
  $convertFromMarkdownString,
  $convertToMarkdownString,
  TRANSFORMERS,
} from "@lexical/markdown";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import { LexicalComposer } from "@lexical/react/LexicalComposer";
import { ContentEditable } from "@lexical/react/LexicalContentEditable";
import { LexicalErrorBoundary } from "@lexical/react/LexicalErrorBoundary";
import { HistoryPlugin } from "@lexical/react/LexicalHistoryPlugin";
import { LinkPlugin } from "@lexical/react/LexicalLinkPlugin";
import { ListPlugin } from "@lexical/react/LexicalListPlugin";
import { MarkdownShortcutPlugin } from "@lexical/react/LexicalMarkdownShortcutPlugin";
import { OnChangePlugin } from "@lexical/react/LexicalOnChangePlugin";
import { RichTextPlugin } from "@lexical/react/LexicalRichTextPlugin";
import { HeadingNode, QuoteNode } from "@lexical/rich-text";
import type { LexicalEditor } from "lexical";
import { useEffect } from "react";

const theme = {
  heading: {
    h1: "tx-md-h1",
    h2: "tx-md-h2",
    h3: "tx-md-h3",
  },
  text: {
    bold: "tx-md-bold",
    italic: "tx-md-italic",
    code: "tx-md-code",
  },
  list: {
    ul: "tx-md-ul",
    ol: "tx-md-ol",
    listitem: "tx-md-li",
  },
  link: "tx-md-link",
  quote: "tx-md-quote",
  paragraph: "tx-md-p",
};

function MarkdownInit({ markdown }: { markdown: string }) {
  const [editor] = useLexicalComposerContext();
  // useEffect with [] runs once. editor.update mutates the editor
  // state synchronously so the seed is applied before the first
  // paint.
  useEffect(() => {
    editor.update(() => $convertFromMarkdownString(markdown, TRANSFORMERS));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}

type Props = {
  initialMarkdown?: string;
  onChange?: (md: string) => void;
};

export function MarkdownEditor({
  initialMarkdown = "",
  onChange,
}: Props) {
  const config = {
    namespace: "TeraxlystMarkdownEditor",
    theme,
    nodes: [
      HeadingNode,
      QuoteNode,
      ListNode,
      ListItemNode,
      LinkNode,
      CodeNode,
    ],
    onError: (err: Error) => {
      console.error("[teraxlyst] lexical error:", err);
    },
  };

  const handleChange = (_state: unknown, editor: LexicalEditor) => {
    editor.read(() => {
      onChange?.($convertToMarkdownString(TRANSFORMERS));
    });
  };

  return (
    <LexicalComposer initialConfig={config}>
      <div className="tx-md-shell" data-testid="markdown-editor">
        <RichTextPlugin
          contentEditable={
            <ContentEditable
              className="tx-md-input"
              aria-label="Markdown editor"
              data-testid="markdown-content"
            />
          }
          placeholder={
            <div className="tx-md-placeholder">Start typing markdown...</div>
          }
          ErrorBoundary={LexicalErrorBoundary}
        />
        <HistoryPlugin />
        <ListPlugin />
        <LinkPlugin />
        <MarkdownShortcutPlugin transformers={TRANSFORMERS} />
        <OnChangePlugin onChange={handleChange} />
        <MarkdownInit markdown={initialMarkdown} />
      </div>
    </LexicalComposer>
  );
}
