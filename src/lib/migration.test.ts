// Unit tests for the M3.1 one-time localStorage migration helper.
//
// jsdom (configured in vitest.config.ts) provides a fresh window.localStorage
// per test file. We clear it between cases to avoid cross-test leakage.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { runOneTimeMigration } from "./migration";

const OLD_KEY = "terax-ui-theme-shadow";
const NEW_KEY = "teraxlyst-ui-theme-shadow";
const OLD_KEY_UPDATER = "terax:updater:last-check";
const NEW_KEY_UPDATER = "teraxlyst:updater:last-check";

beforeEach(() => {
  window.localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("runOneTimeMigration", () => {
  it("copies a legacy key value to the new key and removes the original", () => {
    window.localStorage.setItem(OLD_KEY, "dark");

    runOneTimeMigration();

    expect(window.localStorage.getItem(NEW_KEY)).toBe("dark");
    expect(window.localStorage.getItem(OLD_KEY)).toBeNull();
  });

  it("does not overwrite an existing value at the new key", () => {
    window.localStorage.setItem(OLD_KEY, "dark");
    window.localStorage.setItem(NEW_KEY, "light");

    runOneTimeMigration();

    // The newer value wins. The legacy key is still cleared so a future run
    // is a no-op.
    expect(window.localStorage.getItem(NEW_KEY)).toBe("light");
    expect(window.localStorage.getItem(OLD_KEY)).toBeNull();
  });

  it("is idempotent on a second call", () => {
    window.localStorage.setItem(OLD_KEY, "dark");
    window.localStorage.setItem(OLD_KEY_UPDATER, "1700000000");

    runOneTimeMigration();
    runOneTimeMigration();

    expect(window.localStorage.getItem(NEW_KEY)).toBe("dark");
    expect(window.localStorage.getItem(NEW_KEY_UPDATER)).toBe("1700000000");
    expect(window.localStorage.getItem(OLD_KEY)).toBeNull();
    expect(window.localStorage.getItem(OLD_KEY_UPDATER)).toBeNull();
  });

  it("swallows a storage exception and does not throw", () => {
    window.localStorage.setItem(OLD_KEY, "dark");
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const setItemSpy = vi
      .spyOn(window.localStorage.__proto__, "setItem")
      .mockImplementation(() => {
        throw new Error("QuotaExceeded");
      });

    // The helper must not propagate the throw; boot must continue.
    expect(() => runOneTimeMigration()).not.toThrow();
    expect(warnSpy).toHaveBeenCalledTimes(1);
    expect(warnSpy.mock.calls[0][0]).toContain("migration: failed");

    setItemSpy.mockRestore();
  });

  it("does nothing when there is no legacy key to migrate", () => {
    const infoSpy = vi.spyOn(console, "info").mockImplementation(() => {});

    runOneTimeMigration();

    expect(window.localStorage.length).toBe(0);
    expect(infoSpy).not.toHaveBeenCalled();
  });
});
