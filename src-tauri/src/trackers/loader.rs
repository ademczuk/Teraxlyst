// YAML loader: reads `<workspace>/.teraxlyst/trackers/*.yaml`, parses each
// into a TrackerDef, validates it, and returns the validated set.
//
// Failure policy: per the M4 spec, an invalid YAML file produces a warning
// log line and is skipped. We do not fail the whole load on one bad file;
// the user's other trackers must remain usable. The caller can surface a
// list of bad-file diagnostics via the returned `errors` vec for the UI.

use std::path::{Path, PathBuf};

use super::error::TrackerError;
use super::schema::TrackerDef;

const TRACKERS_SUBDIR: &str = ".teraxlyst/trackers";

#[derive(Debug)]
pub struct LoadReport {
    pub trackers: Vec<TrackerDef>,
    pub errors: Vec<LoadErrorEntry>,
}

#[derive(Debug)]
pub struct LoadErrorEntry {
    pub path: PathBuf,
    pub message: String,
}

// Read every *.yaml file in <workspace>/.teraxlyst/trackers/. Missing
// directory returns an empty report (workspaces don't have to define
// trackers). Sub-directories are not recursed - tracker files live flat.
pub fn load_trackers_from_dir(workspace_path: &Path) -> Result<LoadReport, TrackerError> {
    let dir = workspace_path.join(TRACKERS_SUBDIR);
    let mut report = LoadReport {
        trackers: Vec::new(),
        errors: Vec::new(),
    };

    if !dir.exists() {
        log::info!(
            "tracker directory {} does not exist; nothing to load",
            dir.display()
        );
        return Ok(report);
    }

    let entries = std::fs::read_dir(&dir)?;
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::warn!("failed to read tracker dir entry: {}", e);
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.eq_ignore_ascii_case("yaml") || s.eq_ignore_ascii_case("yml"))
            .unwrap_or(false);
        if !is_yaml {
            continue;
        }
        match load_single(&path) {
            Ok(def) => report.trackers.push(def),
            Err(e) => {
                log::warn!(
                    "skipping invalid tracker yaml {}: {}",
                    path.display(),
                    e
                );
                report.errors.push(LoadErrorEntry {
                    path: path.clone(),
                    message: e.to_string(),
                });
            }
        }
    }
    Ok(report)
}

// Parse one YAML file into a validated TrackerDef. Used by the dir loader
// and by tests. Visible to mod-level so tests in tests.rs can call it
// directly without writing to disk.
pub fn load_single(path: &Path) -> Result<TrackerDef, TrackerError> {
    let text = std::fs::read_to_string(path)?;
    parse_yaml(&text)
}

// Pure parse + validate over a YAML string. Exposed for tests so they
// don't need filesystem fixtures.
pub fn parse_yaml(text: &str) -> Result<TrackerDef, TrackerError> {
    let def: TrackerDef = serde_yaml_ng::from_str(text)?;
    def.validate()?;
    Ok(def)
}
