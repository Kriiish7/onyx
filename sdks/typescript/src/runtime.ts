/**
 * Runtime detection utilities for cross-platform compatibility.
 *
 * The Onyx SDK works across Node.js, Bun, Deno, and browsers out of the box.
 * These helpers let you inspect the current runtime if needed.
 *
 * @module
 */

/** Supported JavaScript runtime environments. */
export type Runtime = "node" | "bun" | "deno" | "browser" | "unknown";

/**
 * Detect the current JavaScript runtime.
 *
 * @returns The detected {@link Runtime} identifier.
 *
 * @example
 * ```typescript
 * import { detectRuntime } from "@onyx/sdk";
 *
 * switch (detectRuntime()) {
 *   case "deno":
 *     console.log("Running in Deno");
 *     break;
 *   case "bun":
 *     console.log("Running in Bun");
 *     break;
 *   case "node":
 *     console.log("Running in Node.js");
 *     break;
 *   case "browser":
 *     console.log("Running in a browser");
 *     break;
 * }
 * ```
 */
export function detectRuntime(): Runtime {
  // Deno exposes a global `Deno` namespace
  if (typeof globalThis !== "undefined" && "Deno" in globalThis) {
    return "deno";
  }

  // Bun exposes a global `Bun` object
  if (typeof globalThis !== "undefined" && "Bun" in globalThis) {
    return "bun";
  }

  // Node.js has `process.versions.node`
  if (
    typeof globalThis !== "undefined" &&
    "process" in globalThis &&
    typeof (globalThis as unknown as Record<string, unknown>).process ===
      "object"
  ) {
    const proc = (globalThis as unknown as Record<string, unknown>).process as
      | { versions?: { node?: string } }
      | undefined;
    if (proc?.versions?.node) {
      return "node";
    }
  }

  // Browser environments have `window` and `document`
  if (typeof globalThis !== "undefined" && "document" in globalThis) {
    return "browser";
  }

  return "unknown";
}

/**
 * Check whether the current runtime is server-side (Node.js, Bun, or Deno).
 *
 * @example
 * ```typescript
 * import { isServer } from "@onyx/sdk";
 *
 * if (isServer()) {
 *   // Safe to use server-only features
 * }
 * ```
 */
export function isServer(): boolean {
  const rt = detectRuntime();
  return rt === "node" || rt === "bun" || rt === "deno";
}

/**
 * Check whether the current runtime is a browser environment.
 */
export function isBrowser(): boolean {
  return detectRuntime() === "browser";
}
