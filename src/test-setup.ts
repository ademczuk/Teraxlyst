// Vitest setup file. Loaded once per test worker before any test runs.
//
// Brings in jest-dom matchers (toBeInTheDocument, toHaveTextContent, etc.) so
// individual test files don't need to import them. The /vitest entry wires the
// matchers into vitest's expect rather than jest's.

import "@testing-library/jest-dom/vitest";
