# Example tracker definitions

Drop any of these YAML files in your workspace at
`.teraxlyst/trackers/<name>.yaml` to enable the corresponding tracker.

The Teraxlyst app calls `tracker_load_workspace` on the workspace path,
which reads every `*.yaml` under `.teraxlyst/trackers/`, validates the
schema, and syncs the parsed definition to the workspace SQLite DB.

## What ships here

| File | Purpose |
|------|---------|
| `bugs.yaml` | Bug reports with severity, status, reporter, assignee, tags. |
| `tasks.yaml` | Generic task tracking with priority, assignee, due date, dependencies. |
| `decisions.yaml` | ADR-style architecture decisions. Status moves proposed -> accepted -> deprecated. |
| `plans.yaml` | Multi-phase initiatives with progress, start/target dates, milestones, risks. |

## Field type cheatsheet

Per `planning/ARCHITECTURE.md §7`:

- `string` - single line of text. Use for titles and short labels.
- `text` - multiline. Use for descriptions, notes, ADR context blocks.
- `number` - integer or float. Use for estimates, progress (0-100), counts.
- `select` - single choice from `options:` list.
- `multiselect` - zero or more choices from `options:` list.
- `date` / `datetime` - ISO 8601 strings. Set `auto: created_at | updated_at` for timestamps the runtime maintains.
- `boolean` - true/false.
- `user` - reference to a user identity.
- `reference` - reference to another tracker item by public_id (e.g. `BUG-014`).
- `array` - list of arbitrary values.
- `object` - nested JSON object.

## Roles

Roles are semantic markers Teraxlyst uses to render kanban columns,
sort items, surface assignees, etc. Each role can appear on AT MOST one
field per tracker:

- `title` - displayed in lists and as the card heading.
- `workflowStatus` - the field that drives kanban columns (must be `select`).
- `priority` - displayed as a colored chip.
- `assignee`, `reporter` - user fields.
- `tags` - chip-style multiselect.
- `startDate`, `dueDate` - shown in deadline view.
- `progress` - 0-100 number, rendered as a progress bar.

## ID format

| Type | Example | Use when |
|------|---------|----------|
| `sequential` (prefix + pad) | `BUG-014` | You want predictable, human-readable IDs. |
| `ulid` | `01HXZ3X4Q3...` | You want sortable, globally unique IDs without coordination. |
| `uuid` | `f47ac10b-58cc...` | You want maximum uniqueness, e.g. for federation. |

## Creating items from inside Teraxlyst

An AI agent running inside Teraxlyst can create + update items via the
in-process MCP tools:

```
tracker_create_item({ tracker: "Bugs", fields: { title: "...", severity: "high", status: "open", description: "..." } })
tracker_update_item({ id: 42, fields: { status: "fixed" } })
tracker_list_items({ tracker: "Bugs", status: "open" })
tracker_query({ workspace_id: 1, tracker_name: "Bugs", query: { tags: ["regression"] } })
```

## Customizing

Copy any of these files, rename, edit. The file watcher (M4.2) will be
added in a future patch so that edits to YAML files automatically
re-sync to the workspace DB. For now, restart Teraxlyst (or call
`tracker_load_workspace` again) after editing a tracker definition.
