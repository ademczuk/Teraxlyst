# Example Teraxlyst workspace

Drop this directory anywhere on your filesystem, open it as a Teraxlyst workspace, and the app picks up the pre-populated tracker definitions in `.teraxlyst/trackers/`.

## What's in here

```
examples/workspace/
  .teraxlyst/
    trackers/
      bugs.yaml       # severity / status / reporter / assignee / tags
      tasks.yaml      # priority / due_date / dependencies
      decisions.yaml  # ADR-style proposed/accepted/deprecated/superseded
      plans.yaml      # multi-phase initiatives with progress + milestones
  src/                # placeholder for your own code
  README.md           # this file
```

## How to use

1. Copy or symlink this whole directory to a real location (Teraxlyst's PTY workspace becomes the new dir).
2. Open Teraxlyst, hit the "Trackers" tab in the sidebar.
3. Click "Load workspace trackers" or restart Teraxlyst. The four trackers above appear.
4. Click any tracker to see its TrackerTable. Empty by default.
5. Create items either through the UI or by having an AI agent inside Teraxlyst call `tracker_create_item` via the M3 MCP host.

## Customizing

Each YAML file is a tracker definition. Edit, copy, or add new files in `.teraxlyst/trackers/`. The schema cheatsheet is in [`examples/trackers/README.md`](../trackers/README.md) at the repo root.

## What's NOT in here

- No example sessions. Sessions are runtime-only.
- No example diff proposals. Same reason.
- No API key configuration. API keys live in the OS keyring per workspace; the app's Settings -> Models dialog is where you set them.

## Want to scratch around without committing

Copy this directory to `~/teraxlyst-scratch` (or wherever), open as workspace, experiment. Teraxlyst's local SQLite lives at `<app_data_dir>/teraxlyst.sqlite` (not in the workspace), so deleting the scratch workspace removes only its tracker YAMLs and any project source you put in `src/`.
