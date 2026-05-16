// TrackersPanel: sidebar panel that loads YAML-defined trackers for the
// active workspace and lets the user drill into a per-tracker item list.
//
// State machine:
//   "list"  -> show the list of trackers; click a row to drill in.
//   "table" -> show items for the selected tracker; Back returns to list.
//
// We pull workspaces via db_list_workspaces (same shape SessionList uses)
// and pick workspaces[0] as the active one — multi-workspace switching is
// a later milestone. tracker_load_workspace parses the YAML and syncs to
// the DB; tracker_list_items returns the rows for a chosen tracker.
//
// The TrackerTable subcomponent renders items in a compact table. Columns
// are derived from the tracker's field defs (capped at 4) so the sidebar
// stays readable; full editing UI is M4.x.

import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft01Icon, Refresh01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";

type WorkspaceLite = { id: number; name: string; path: string };

type FieldDef = {
  name: string;
  type: string;
  roles?: string[];
  options?: string[] | null;
  required?: boolean;
};

type TrackerDef = {
  name: string;
  id_format: unknown;
  fields: FieldDef[];
};

type LoadedTrackerDto = {
  id: number;
  name: string;
  schema: TrackerDef;
};

type LoadErrorDto = {
  path: string;
  message: string;
};

type LoadWorkspaceResult = {
  trackers: LoadedTrackerDto[];
  errors: LoadErrorDto[];
};

type TrackerItem = {
  id: number;
  tracker_id: number;
  public_id: string;
  status: string | null;
  fields_json: string;
  created_at: number;
  updated_at: number;
};

// Cap how many schema fields we render in the compact sidebar table.
const MAX_COLUMNS = 4;

function TrackerList({
  trackers,
  errors,
  onSelect,
}: {
  trackers: LoadedTrackerDto[];
  errors: LoadErrorDto[];
  onSelect: (t: LoadedTrackerDto) => void;
}) {
  if (trackers.length === 0 && errors.length === 0) {
    return (
      <div className="px-3 py-2 text-xs italic text-muted-foreground">
        no trackers in this workspace
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-2 p-2">
      {trackers.length > 0 ? (
        <ul className="flex flex-col gap-1">
          {trackers.map((t) => (
            <li key={t.id}>
              <button
                type="button"
                onClick={() => onSelect(t)}
                className="block w-full rounded border border-border bg-background px-2 py-1.5 text-left text-xs hover:bg-muted/40"
              >
                <span className="block truncate font-medium">{t.name}</span>
                <span className="block text-[10px] text-muted-foreground">
                  {t.schema.fields.length} fields
                </span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
      {errors.length > 0 ? (
        <div className="flex flex-col gap-1">
          <h3 className="text-[10px] font-semibold uppercase tracking-wide text-destructive">
            Parse errors ({errors.length})
          </h3>
          <ul className="flex flex-col gap-1">
            {errors.map((e, i) => (
              <li
                key={`${e.path}-${i}`}
                className="rounded border border-destructive/40 bg-destructive/5 px-2 py-1 text-[10px] text-destructive"
                title={e.path}
              >
                <div className="truncate font-mono">{e.path}</div>
                <div>{e.message}</div>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}

function TrackerTable({ tracker }: { tracker: LoadedTrackerDto }) {
  const [items, setItems] = useState<TrackerItem[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await invoke<TrackerItem[]>("tracker_list_items", {
        args: { tracker_id: tracker.id, status: null },
      });
      setItems(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [tracker.id]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const columns = useMemo(() => tracker.schema.fields.slice(0, MAX_COLUMNS), [
    tracker.schema.fields,
  ]);

  const rows = useMemo(() => {
    return items.map((item) => {
      let fields: Record<string, unknown> = {};
      try {
        const parsed: unknown = JSON.parse(item.fields_json);
        if (parsed && typeof parsed === "object") {
          fields = parsed as Record<string, unknown>;
        }
      } catch {
        // Fall through with empty fields; the row will show blanks.
      }
      return { item, fields };
    });
  }, [items]);

  return (
    <div className="flex h-full min-h-0 flex-col">
      {error ? (
        <div className="border-b border-destructive bg-destructive/10 px-2 py-1 text-xs text-destructive">
          {error}
        </div>
      ) : null}
      <div className="min-h-0 flex-1 overflow-auto">
        {loading && items.length === 0 ? (
          <div className="p-2 text-xs italic text-muted-foreground">
            loading...
          </div>
        ) : items.length === 0 ? (
          <div className="p-2 text-xs italic text-muted-foreground">
            no items yet
          </div>
        ) : (
          <table className="w-full table-auto text-xs">
            <thead className="bg-muted/30 text-left text-[10px] uppercase tracking-wide text-muted-foreground">
              <tr>
                <th className="px-2 py-1">id</th>
                {columns.map((c) => (
                  <th key={c.name} className="px-2 py-1">
                    {c.name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map(({ item, fields }) => (
                <tr key={item.id} className="border-t border-border/60">
                  <td className="px-2 py-1 font-mono">{item.public_id}</td>
                  {columns.map((c) => {
                    const raw = fields[c.name];
                    const display =
                      raw === undefined || raw === null
                        ? ""
                        : typeof raw === "string"
                          ? raw
                          : JSON.stringify(raw);
                    return (
                      <td
                        key={c.name}
                        className="max-w-[120px] truncate px-2 py-1"
                        title={display}
                      >
                        {display}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

export default function TrackersPanel() {
  const [workspace, setWorkspace] = useState<WorkspaceLite | null>(null);
  const [trackers, setTrackers] = useState<LoadedTrackerDto[]>([]);
  const [errors, setErrors] = useState<LoadErrorDto[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [selected, setSelected] = useState<LoadedTrackerDto | null>(null);

  const loadAll = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const list = await invoke<WorkspaceLite[]>("db_list_workspaces");
      const active = list[0] ?? null;
      setWorkspace(active);
      if (!active) {
        setTrackers([]);
        setErrors([]);
        return;
      }
      const result = await invoke<LoadWorkspaceResult>(
        "tracker_load_workspace",
        {
          workspaceId: active.id,
          workspacePath: active.path,
        },
      );
      setTrackers(result.trackers);
      setErrors(result.errors);
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAll();
  }, [loadAll]);

  return (
    <div className="flex h-full min-h-0 flex-col bg-card">
      <div className="flex items-center gap-2 border-b border-border/60 px-3 py-2">
        {selected ? (
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            onClick={() => setSelected(null)}
            title="Back to trackers"
          >
            <HugeiconsIcon icon={ArrowLeft01Icon} size={14} />
          </Button>
        ) : null}
        <h2 className="flex-1 truncate text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {selected ? selected.name : "Trackers"}
        </h2>
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          onClick={() => void loadAll()}
          title="Refresh"
          disabled={loading}
        >
          <HugeiconsIcon icon={Refresh01Icon} size={14} />
        </Button>
      </div>

      {loadError ? (
        <div className="border-b border-destructive bg-destructive/10 px-2 py-1 text-xs text-destructive">
          {loadError}
        </div>
      ) : null}

      {!workspace && !loading ? (
        <div className="px-3 py-2 text-xs italic text-muted-foreground">
          no workspace available
        </div>
      ) : null}

      <div className="min-h-0 flex-1 overflow-hidden">
        {selected ? (
          <TrackerTable tracker={selected} />
        ) : (
          <div className="h-full overflow-auto">
            <TrackerList
              trackers={trackers}
              errors={errors}
              onSelect={setSelected}
            />
          </div>
        )}
      </div>
    </div>
  );
}
