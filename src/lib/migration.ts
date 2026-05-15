// One-time migration from the legacy `terax-*` / `terax:*` localStorage key
// prefixes to the rebranded `teraxlyst-*` / `teraxlyst:*` prefixes.
//
// Scope: localStorage only. The tauri-plugin-store JSON files (which live on
// disk in the app data dir, e.g. `terax-settings.json`) are NOT migrated by
// this helper. Renaming the `STORE_PATH` constants in their respective
// modules is sufficient: tauri-plugin-store will recreate each file at first
// read. Any pre-v0.1 data sitting in the old JSON files is silently orphaned.
// That trade-off is acceptable here because the project has zero users.
//
// This helper is idempotent: subsequent calls find no legacy keys, log
// nothing, and return immediately. It is wrapped in try/catch so a
// localStorage permission error (e.g. private-mode browsers, sandboxed
// contexts) cannot kill app boot.

// Each tuple is [old key, new key]. Keep the list explicit rather than
// auto-derived so an accidental rename in one place does not silently
// reshape the migration semantics.
const KEY_PAIRS: ReadonlyArray<readonly [string, string]> = [
  ["terax-ui-theme-shadow", "teraxlyst-ui-theme-shadow"],
  ["terax:updater:last-check", "teraxlyst:updater:last-check"],
];

export function runOneTimeMigration(): void {
  if (typeof window === "undefined") return;
  try {
    const ls = window.localStorage;
    for (const [oldKey, newKey] of KEY_PAIRS) {
      const oldVal = ls.getItem(oldKey);
      if (oldVal === null) continue;
      // Never clobber a value at the new key. If the new key already exists,
      // just drop the old one: the new write path has been used since the
      // rebrand and its value is authoritative.
      const newVal = ls.getItem(newKey);
      if (newVal === null) {
        ls.setItem(newKey, oldVal);
        console.info(`[teraxlyst] migration: ran key=${oldKey}`);
      }
      ls.removeItem(oldKey);
    }
  } catch (e) {
    // Defensive: don't let a storage exception cascade into a white screen.
    console.warn("[teraxlyst] migration: failed", e);
  }
}
