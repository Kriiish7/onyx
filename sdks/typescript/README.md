# Onyx SDK for TypeScript / JavaScript

Official TypeScript/JavaScript client library for the **Onyx** AI infrastructure engine — combining semantic search, knowledge graphs, and temporal versioning in a graph-native vector memory system.

Works natively with **Node.js**, **Bun**, **Deno**, browsers, and edge runtimes.

## Installation

### Node.js

```bash
npm install @onyx/sdk
# or
yarn add @onyx/sdk
# or
pnpm add @onyx/sdk
```

### Bun

```bash
bun add @onyx/sdk
```

### Deno

Import directly from source — no build step required:

```typescript
// Import from a local clone / vendored copy
import { OnyxClient } from "./sdks/typescript/src/index.ts";

// Or from a URL (e.g. GitHub raw / your registry)
import { OnyxClient } from "https://raw.githubusercontent.com/your-org/onyx/main/sdks/typescript/src/index.ts";
```

If you publish to [JSR](https://jsr.io), users can also:

```typescript
import { OnyxClient } from "jsr:@onyx/sdk";
```

> **Note:** The `deno.json` file at the SDK root provides task shortcuts and an import map for local development.

**Requirements:**

- Node.js ≥ 18, Bun ≥ 1.0, or Deno ≥ 1.38
- All runtimes must support the Web Fetch API (`fetch`, `Request`, `Response`, `AbortController`)

## Quick Start

### Node.js / Bun

```typescript
import { OnyxClient } from "@onyx/sdk";

const client = new OnyxClient("http://localhost:3000", {
  apiKey: "your-api-key",
});

// Health check
const healthy = await client.health();
console.log("Server healthy:", healthy);

// Create a node
const node = await client.nodes.create({
  name: "hello_world",
  content: 'fn hello() { println!("hello"); }',
  nodeType: { type: "CodeEntity", kind: "Function" },
});
console.log("Created node:", node.id);

// Semantic search
const results = await client.search.query({
  embedding: [0.1, 0.2, 0.3],
  topK: 5,
});
for (const item of results.items) {
  console.log(`  ${item.name} (score: ${item.score.toFixed(3)})`);
}
```

### Deno

```typescript
import { OnyxClient } from "./sdks/typescript/src/index.ts";

const client = new OnyxClient("http://localhost:3000", {
  apiKey: Deno.env.get("ONYX_API_KEY"),
});

const healthy = await client.health();
console.log("Server healthy:", healthy);

const node = await client.nodes.create({
  name: "hello_world",
  content: 'fn hello() { println!("hello"); }',
  nodeType: { type: "CodeEntity", kind: "Function" },
});
console.log("Created node:", node.id);
```

Run with:

```bash
deno run --allow-net --allow-env main.ts
```

### Bun (script mode)

```typescript
// main.ts — run with `bun run main.ts`
import { OnyxClient, detectRuntime } from "@onyx/sdk";

console.log(`Running on ${detectRuntime()}`); // "bun"

const client = new OnyxClient("http://localhost:3000", {
  apiKey: Bun.env.ONYX_API_KEY,
});

const healthy = await client.health();
console.log("Server healthy:", healthy);
```

## Client Options

```typescript
const client = new OnyxClient("http://localhost:3000", {
  apiKey: "sk-...", // Authorization: Bearer <key>
  timeout: 60_000, // Request timeout in ms (default: 30 000)
  fetch: customFetchFn, // Custom fetch implementation
});
```

## Sub-Clients

| Sub-client      | Property         | Description                 |
| --------------- | ---------------- | --------------------------- |
| `NodesClient`   | `client.nodes`   | Node CRUD operations        |
| `EdgesClient`   | `client.edges`   | Edge CRUD operations        |
| `SearchClient`  | `client.search`  | Vector similarity search    |
| `HistoryClient` | `client.history` | Version history & branching |
| `IngestClient`  | `client.ingest`  | Code ingestion pipeline     |
| `BillingClient` | `client.billing` | Stripe billing integration  |

## Node Operations

```typescript
import type { CreateNodeRequest, UpdateNodeRequest } from "@onyx/sdk";

// Create
const node = await client.nodes.create({
  name: "MyStruct",
  content: "pub struct MyStruct { ... }",
  nodeType: { type: "CodeEntity", kind: "Struct" },
  provenance: { filePath: "src/lib.rs", lineRange: [10, 20] },
});

// Read
const fetched = await client.nodes.get(node.id);

// Update
const updated = await client.nodes.update(node.id, {
  name: "RenamedStruct",
});

// Delete
await client.nodes.delete(node.id);

// List (paginated)
const page = await client.nodes.list(1, 50);
console.log(`Total nodes: ${page.total}`);

// Subgraph extraction
const sub = await client.nodes.subgraph(node.id, 2);
```

## Edge Operations

```typescript
const edge = await client.edges.create({
  edgeType: "Calls",
  sourceId: sourceId,
  targetId: targetId,
  confidence: 0.95,
});

await client.edges.delete(edge.id);
```

## Semantic Search

```typescript
const results = await client.search.query({
  embedding: embeddingVector,
  topK: 10,
  maxDepth: 3,
  edgeTypes: ["Calls", "Imports"],
  includeHistory: true,
  minConfidence: 0.5,
});

console.log(`Found ${results.items.length} items in ${results.queryTimeMs}ms`);

for (const item of results.items) {
  console.log(
    `  ${item.name} (score=${item.score.toFixed(3)}, via=${item.source})`,
  );
}
```

## Version History

```typescript
// List versions for an entity
const versions = await client.history.listVersions(entityId);

// Get content at a specific version
const content = await client.history.getContentAtVersion(entityId, "v-abc123");

// Create a branch
const branch = await client.history.createBranch({
  name: "feature/new-api",
  baseVersion: "v-abc123",
});

// Merge branches
const merged = await client.history.mergeBranch({
  source: "feature/new-api",
  target: "main",
});
```

## Code Ingestion

```typescript
// Single unit
const result = await client.ingest.ingestUnit({
  name: "process_data",
  content: "pub fn process_data(input: &str) -> Result<()> { ... }",
  kind: "Function",
  language: "Rust",
  filePath: "src/processing.rs",
  lineRange: [42, 58],
  signature: "pub fn process_data(input: &str) -> Result<()>",
  visibility: "Public",
});

// Batch ingestion
const batch = await client.ingest.ingestCodebase({
  units: [unit1, unit2, unit3],
});
```

## Billing

```typescript
// Checkout session
const session = await client.billing.createCheckoutSession({
  customerEmail: "user@example.com",
  priceId: "price_xxx",
  successUrl: "https://app.example.com/success",
  cancelUrl: "https://app.example.com/cancel",
});
console.log("Checkout URL:", session.url);

// Billing portal
const portal = await client.billing.createBillingPortal({
  customerId: "cus_xxx",
  returnUrl: "https://app.example.com/settings",
});
```

## Error Handling

```typescript
import {
  OnyxError,
  OnyxApiError,
  OnyxNotFoundError,
  OnyxNetworkError,
} from "@onyx/sdk";

try {
  const node = await client.nodes.get("non-existent-id");
} catch (err) {
  if (err instanceof OnyxNotFoundError) {
    console.error("Not found:", err.message);
  } else if (err instanceof OnyxApiError) {
    console.error(`HTTP ${err.statusCode}: ${err.message}`);
  } else if (err instanceof OnyxNetworkError) {
    console.error("Network error:", err.message);
  } else if (err instanceof OnyxError) {
    console.error("SDK error:", err.message);
  }
}
```

| Error class           | Description                                             |
| --------------------- | ------------------------------------------------------- |
| `OnyxError`           | Base class for all SDK errors                           |
| `OnyxApiError`        | HTTP error from the server (has `statusCode`)           |
| `OnyxNotFoundError`   | Resource not found (extends `OnyxApiError`, status 404) |
| `OnyxNetworkError`    | Connection / transport failure                          |
| `OnyxValidationError` | Invalid argument provided to the SDK                    |

## TypeScript Support

The SDK is written in TypeScript and ships with full type definitions. All request/response interfaces are exported from the package root:

```typescript
import type {
  Node,
  Edge,
  SearchRequest,
  SearchResponse,
  CreateNodeRequest,
  // ... all other types
} from "@onyx/sdk";
```

## Platform Support

| Platform           | Version | Status       | Notes                                         |
| ------------------ | ------- | ------------ | --------------------------------------------- |
| Node.js            | ≥ 18    | Full support | Uses native `fetch` (Node 18+)                |
| Bun                | ≥ 1.0   | Full support | Resolves TS source directly — no build needed |
| Deno               | ≥ 1.38  | Full support | Import from source `.ts` files or via JSR     |
| Browsers           | Modern  | Full support | ESM bundle via `dist/index.mjs`               |
| Cloudflare Workers | —       | Full support | Pass custom `fetch` if needed                 |
| Vercel Edge        | —       | Full support | Standard Web API compatible                   |

### How It Works

The SDK uses only **Web-standard APIs** (`fetch`, `Request`, `Response`, `AbortController`, `setTimeout`) — no Node.js-specific modules like `http`, `https`, `fs`, or `Buffer`. This means it runs anywhere these APIs are available.

- **Bun** resolves the `"bun"` export condition in `package.json`, importing TypeScript source directly for zero-overhead usage.
- **Deno** can import the `.ts` source files directly — no compilation or bundling step required.
- **Node.js** uses the pre-built `dist/` artifacts (CJS or ESM).

### Custom Fetch

All runtimes allow injecting a custom `fetch` implementation:

```typescript
const client = new OnyxClient("http://localhost:3000", {
  fetch: myCustomFetch, // e.g. undici, node-fetch, or a test mock
});
```

## Runtime Detection

The SDK exports utilities for detecting the current runtime:

```typescript
import { detectRuntime, isServer, isBrowser } from "@onyx/sdk";

console.log(detectRuntime()); // "node" | "bun" | "deno" | "browser" | "unknown"
console.log(isServer()); // true for Node.js, Bun, Deno
console.log(isBrowser()); // true in browser environments
```

This is useful for conditional logic like choosing a storage backend or logging transport.

## License

MIT — see [LICENSE](../../LICENSE) for details.
