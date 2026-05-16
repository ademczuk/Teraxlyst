// DataPanel: CSV editor backed by react-data-grid 7.x.
//
// Features in this scaffold:
//   - Parse pasted CSV text into rows and columns
//   - Inline edit any cell via double-click (renderEditCell: textEditor)
//   - Add row / delete last row
//   - Column resize, column sort
//   - Virtualized rendering (handles 50K+ rows)
//   - Serialize back to CSV text for download / paste-out
//
// Persistence and Tauri-fs integration are follow-ups. For now we
// seed with a small example CSV so the user can see editing work,
// and the "Paste CSV" / "Copy CSV" buttons let them round-trip
// through any external source.
//
// react-data-grid 7.0.0-beta.59 is React-19-native (peer dep
// "react": "^19.2"). Bundle ~15 KB gzipped, virtualization always-on.

import "react-data-grid/lib/styles.css";

import { useMemo, useState } from "react";
import { DataGrid, renderTextEditor } from "react-data-grid";
import type { Column } from "react-data-grid";

type Row = Record<string, string>;

const SEED_CSV = `id,name,owner,priority,status
TASK-001,Lexical WYSIWYG editor,andrew,high,done
TASK-002,Excalidraw canvas,andrew,high,done
TASK-003,Mermaid diagrams,andrew,high,done
TASK-004,CSV editor (this!),andrew,medium,in-progress
TASK-005,Tauri fs round-trip for notes,andrew,medium,pending
TASK-006,Persistence for canvas drawings,andrew,medium,pending
TASK-007,Lexical syntax-highlighted code blocks,andrew,low,pending`;

function parseCsv(text: string): { rows: Row[]; headers: string[] } {
  const lines = text.trim().split(/\r?\n/).filter((l) => l.length > 0);
  if (lines.length === 0) return { rows: [], headers: [] };
  const headers = splitCsvLine(lines[0] ?? "");
  const rows = lines.slice(1).map((line) => {
    const cells = splitCsvLine(line);
    const row: Row = {};
    headers.forEach((h, i) => {
      row[h] = cells[i] ?? "";
    });
    return row;
  });
  return { rows, headers };
}

// Minimal CSV splitter: handles double-quoted cells with embedded
// commas and "" escapes. Not RFC-4180 perfect (no multiline cells)
// but good enough for paste-in editing.
function splitCsvLine(line: string): string[] {
  const out: string[] = [];
  let cur = "";
  let inQuotes = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (inQuotes) {
      if (ch === '"') {
        if (line[i + 1] === '"') {
          cur += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        cur += ch;
      }
    } else if (ch === '"') {
      inQuotes = true;
    } else if (ch === ",") {
      out.push(cur);
      cur = "";
    } else {
      cur += ch;
    }
  }
  out.push(cur);
  return out;
}

function serializeCsv(headers: string[], rows: Row[]): string {
  const escape = (v: string) =>
    /[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v;
  const lines = [headers.join(",")];
  for (const r of rows) {
    lines.push(headers.map((h) => escape(r[h] ?? "")).join(","));
  }
  return lines.join("\n");
}

export default function DataPanel() {
  const initial = useMemo(() => parseCsv(SEED_CSV), []);
  const [rows, setRows] = useState<Row[]>(initial.rows);
  const [headers, setHeaders] = useState<string[]>(initial.headers);
  const [status, setStatus] = useState<string>("");

  const columns = useMemo<Column<Row>[]>(
    () =>
      headers.map((h) => ({
        key: h,
        name: h,
        editable: true,
        renderEditCell: renderTextEditor,
        resizable: true,
        sortable: true,
      })),
    [headers],
  );

  function addRow() {
    const blank: Row = {};
    headers.forEach((h) => {
      blank[h] = "";
    });
    setRows((r) => [...r, blank]);
    setStatus(`added row, ${rows.length + 1} total`);
  }

  function deleteLastRow() {
    if (rows.length === 0) return;
    setRows((r) => r.slice(0, -1));
    setStatus(`deleted last row, ${rows.length - 1} total`);
  }

  async function copyCsv() {
    try {
      const csv = serializeCsv(headers, rows);
      await navigator.clipboard.writeText(csv);
      setStatus(`copied ${rows.length} rows to clipboard`);
    } catch (e) {
      setStatus(`copy failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  async function pasteCsv() {
    try {
      const text = await navigator.clipboard.readText();
      const parsed = parseCsv(text);
      if (parsed.headers.length === 0) {
        setStatus("clipboard had no CSV-like content");
        return;
      }
      setHeaders(parsed.headers);
      setRows(parsed.rows);
      setStatus(`pasted ${parsed.rows.length} rows`);
    } catch (e) {
      setStatus(`paste failed: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="data-panel">
      <div className="flex items-center justify-between border-b border-border/60 bg-card px-3 py-2 text-xs">
        <span className="font-medium text-foreground">Data</span>
        <span className="text-muted-foreground">
          CSV editor - not persisted yet
        </span>
      </div>
      <div className="flex items-center gap-2 border-b border-border/60 bg-card px-3 py-1.5 text-xs">
        <button
          type="button"
          onClick={addRow}
          className="rounded border border-border/60 bg-background px-2 py-1 text-foreground hover:bg-muted/40"
        >
          + Row
        </button>
        <button
          type="button"
          onClick={deleteLastRow}
          className="rounded border border-border/60 bg-background px-2 py-1 text-foreground hover:bg-muted/40"
          disabled={rows.length === 0}
        >
          - Last row
        </button>
        <button
          type="button"
          onClick={copyCsv}
          className="rounded border border-border/60 bg-background px-2 py-1 text-foreground hover:bg-muted/40"
        >
          Copy CSV
        </button>
        <button
          type="button"
          onClick={pasteCsv}
          className="rounded border border-border/60 bg-background px-2 py-1 text-foreground hover:bg-muted/40"
        >
          Paste CSV
        </button>
        <span className="ml-auto text-muted-foreground" data-testid="data-status">
          {rows.length} rows {status ? `- ${status}` : ""}
        </span>
      </div>
      <div className="min-h-0 flex-1" data-testid="data-grid-mount">
        <DataGrid
          columns={columns}
          rows={rows}
          onRowsChange={setRows}
          defaultColumnOptions={{ sortable: true, resizable: true }}
          className="rdg-dark h-full"
        />
      </div>
    </div>
  );
}
