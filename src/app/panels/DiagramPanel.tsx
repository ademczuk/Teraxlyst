// DiagramPanel: live Mermaid diagram editor.
//
// Left half: code editor (textarea for now; CodeMirror integration is
// a follow-up). Right half: rendered SVG. The render is debounced 250ms
// so typing doesn't thrash the mermaid worker.
//
// Mermaid is lazy-loaded inside useEffect so the initial app bundle
// stays clean. The dagre layout is the default and does not require
// `script-src 'unsafe-eval'`, so the existing Teraxlyst CSP is
// sufficient.
//
// Persistence: not wired yet. The diagram source lives in component
// state and is lost on tab switch. A follow-up will reuse the M4
// trackers schema to persist named diagrams.

import { useEffect, useRef, useState } from "react";

const DEFAULT_DIAGRAM = `graph TD
  A[Teraxlyst] -->|hosts| B(MCP server)
  A --> C{Provider}
  C -->|stream-json| D[Claude CLI]
  C -->|future| E[Codex / Opencode]
  B --> F[propose_diff]
  B --> G[prompt_for_user_input]
  B --> H[read_workspace_file]`;

export default function DiagramPanel() {
  const [source, setSource] = useState(DEFAULT_DIAGRAM);
  const [error, setError] = useState<string | null>(null);
  const svgRef = useRef<HTMLDivElement>(null);
  const renderIdRef = useRef(0);

  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(async () => {
      try {
        const { default: mermaid } = await import("mermaid");
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: "dark",
          themeVariables: {
            darkMode: true,
            background: "#0f1115",
          },
        });
        // mermaid.render requires a unique id per call. Bump a ref
        // counter to avoid the "element id already used" throw on
        // re-render.
        renderIdRef.current += 1;
        const id = `teraxlyst-mermaid-${renderIdRef.current}`;
        const result = await mermaid.render(id, source);
        if (cancelled) return;
        if (svgRef.current) {
          svgRef.current.innerHTML = result.svg;
        }
        setError(null);
      } catch (e) {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : String(e));
      }
    }, 250);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [source]);

  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="diagram-panel">
      <div className="flex items-center justify-between border-b border-border/60 bg-card px-3 py-2 text-xs">
        <span className="font-medium text-foreground">Mermaid diagram</span>
        <span className="text-muted-foreground">
          dagre layout, dark theme
        </span>
      </div>
      <div className="flex min-h-0 flex-1">
        <textarea
          aria-label="Mermaid source"
          value={source}
          onChange={(e) => setSource(e.target.value)}
          className="h-full w-1/2 resize-none border-r border-border/60 bg-background px-3 py-2 font-mono text-xs text-foreground outline-none placeholder:text-muted-foreground"
          spellCheck={false}
        />
        <div className="relative h-full w-1/2 overflow-auto bg-card">
          {error ? (
            <pre
              className="m-3 whitespace-pre-wrap rounded border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive"
              data-testid="diagram-error"
            >
              {error}
            </pre>
          ) : (
            <div
              ref={svgRef}
              className="flex h-full items-center justify-center p-3 [&_svg]:max-w-full"
              data-testid="diagram-preview"
            />
          )}
        </div>
      </div>
    </div>
  );
}
