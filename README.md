# Onyx

**Graph-Native Vector Memory for AI Agents**

Onyx is a Rust-native infrastructure engine that fuses semantic vector search, structural knowledge graphs, and temporal versioning into a unified retrieval system. It gives AI coding agents the ability to reason about code structure, trace dependencies, analyze impact, and navigate change history through a single query interface.

## Highlights

- Semantic search over code with graph-aware expansion
- Graph traversal for call chains, imports, containment, and test coverage
- Temporal versioning for change history and impact analysis
- Transaction layer that coordinates vector, graph, and history stores
- CLI and interactive REPL for local workflows

## Architecture

```
┌──────────────────────────────────────────────┐
│                Onyx CLI / API                │
│  demo | interactive | ingest | query | ...   │
└───────────────────┬──────────────────────────┘
                    │
     ┌──────────────┼──────────────────┐
     │     Query Engine                │
     │  Vector search → Graph expand   │
     │  → Temporal filter → Fuse       │
     └──────────────┬──────────────────┘
                    │
┌───────────────────┼───────────────────────────┐
│          Transaction Manager                  │
│  (atomic ops across all three stores)         │
├──────────┬──────────────┬─────────────────────┤
│ Vector   │   Graph      │   History           │
│ Store    │   Store      │   Store             │
│ (kNN)    │ (adjacency)  │ (version chains)    │
└──────────┴──────────────┴─────────────────────┘
```

More details:

- Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Data model: [docs/DATA_MODEL.md](docs/DATA_MODEL.md)
- Evaluation plan: [docs/EVALUATION.md](docs/EVALUATION.md)
- Roadmap: [docs/ROADMAP.md](docs/ROADMAP.md)

## Quick Start

### Build

```bash
cargo build
```

### Run the Demo

The demo ingests a small synthetic codebase and runs a few example queries:

```bash
cargo run -- demo
```

### Interactive REPL

```bash
cargo run -- interactive --demo
```

Example commands:

```
onyx> status
onyx> nodes
onyx> edges
onyx> query payment total
onyx> traverse calculate_total --depth 3
onyx> inspect process_payment
onyx> impact apply_discount
onyx> tests calculate_total
onyx> history apply_discount
```

### Ingest Your Own Code

```bash
cargo run -- ingest --path src/your_file.rs

cargo run -- interactive
onyx> ingest src/your_file.rs
onyx> query your search terms
```

## Requirements

- Rust 1.75+ (edition 2021)
- Optional: RocksDB toolchain for persistent storage
- Optional: Docker for consistent build environments

Windows notes: see [docs/WINDOWS_SETUP.md](docs/WINDOWS_SETUP.md).

## Tests

```bash
cargo test
```

## Project Structure

```
src/
├── lib.rs                  # Crate root
├── main.rs                 # CLI (demo, interactive REPL, commands)
├── error.rs                # OnyxError enum, OnyxResult type
├── model/
│   ├── mod.rs              # Re-exports
│   ├── node.rs             # Node, NodeType, Provenance, extensions
│   ├── edge.rs             # Edge, EdgeType, TemporalContext
│   ├── version.rs          # VersionEntry, Diff, Branch
│   └── embedding.rs        # Embedding, BagOfWordsEmbedder
├── store/
│   ├── mod.rs              # Re-exports store traits
│   ├── vector.rs           # VectorStore trait + in-memory impl
│   ├── graph.rs            # GraphStore trait + in-memory impl
│   ├── history.rs          # HistoryStore trait + in-memory impl
│   └── transaction.rs      # Transaction manager
├── query/
│   └── mod.rs              # Query engine pipeline
└── ingest/
    └── mod.rs              # Ingestion engine

docs/
├── ARCHITECTURE.md
├── DATA_MODEL.md
├── EVALUATION.md
└── ROADMAP.md
```

## Status

Current version is a working prototype with in-memory storage and a functional CLI. Persistent storage, advanced embeddings, and multi-language parsing are on the roadmap. See [docs/ROADMAP.md](docs/ROADMAP.md) for planned milestones.

## Contributing

Contributions are welcome. Please read:

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

If you are new, check the issues labeled "good first issue".

## Security

For security concerns, please open a private issue or contact the maintainers directly.

## License

See [LICENSE](LICENSE).
