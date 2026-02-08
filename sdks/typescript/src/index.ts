/**
 * @module @onyx/sdk
 *
 * Official TypeScript/JavaScript SDK for the **Onyx** AI infrastructure engine.
 * Onyx combines semantic search, knowledge graphs, and temporal versioning in
 * a graph-native vector memory system.
 *
 * @example
 * ```typescript
 * import { OnyxClient } from "@onyx/sdk";
 *
 * const client = new OnyxClient("http://localhost:3000", { apiKey: "sk-..." });
 *
 * // Health check
 * const healthy = await client.health();
 *
 * // Create a node
 * const node = await client.nodes.create({
 *   name: "hello_world",
 *   content: 'fn hello() { println!("hello"); }',
 *   nodeType: { type: "CodeEntity", kind: "Function" },
 * });
 *
 * // Search
 * const results = await client.search.query({
 *   embedding: [0.1, 0.2, 0.3],
 *   topK: 5,
 * });
 * ```
 *
 * @packageDocumentation
 */

export { OnyxClient } from "./client";
export type { OnyxClientOptions } from "./client";
export * from "./models";
export * from "./errors";
export { detectRuntime, isServer, isBrowser } from "./runtime";
export type { Runtime } from "./runtime";
