# Onyx Evaluation Plan

## Overview

This document defines how to evaluate Onyx's capabilities across its three core dimensions: semantic search, structural graph reasoning, and temporal versioning. Each evaluation scenario maps to a real-world AI agent use case.

---

## Evaluation Dimensions

### 1. Semantic Search Quality

**Goal**: Verify that vector similarity retrieval surfaces the correct code entities for natural-language queries.

| Prompt | Expected Top Result | Expected in Top 3 |
|--------|--------------------|--------------------|
| "payment processing" | process_payment | calculate_total, record_transaction |
| "calculate price total" | calculate_total | apply_discount, process_payment |
| "discount rules" | get_discount_rules | apply_discount |
| "order validation" | validate_order | validate_item |
| "database transaction" | record_transaction | process_payment |

**Metrics**:
- **Precision@1**: Is the top result the expected one?
- **Recall@3**: Are all expected entities in the top 3?
- **Mean Reciprocal Rank (MRR)**: Average inverse rank of the first correct result.

**Current Limitations**: The BagOfWords embedder is a prototype. Production evaluation should use transformer-based embeddings (e.g., `all-MiniLM-L6-v2`) and compare against BM25 baselines.

---

### 2. Graph Traversal Correctness

**Goal**: Verify structural reasoning traverses the correct call chains, dependencies, and containment hierarchies.

#### Scenario 2a: Call Graph Traceability

```
Prompt: "What functions does calculate_total call?"
Expected: apply_discount -> get_discount_rules
```

```
Prompt: "What calls process_payment?"
Expected: (nothing in current dataset — process_payment is a top-level entry point)
```

```
Prompt: "What is the full call chain from process_payment?"
Expected: process_payment -> calculate_total -> apply_discount -> get_discount_rules
                          -> record_transaction
```

#### Scenario 2b: Impact Analysis

```
Prompt: "If get_discount_rules changes, what is affected?"
Expected: apply_discount (distance 1)
          calculate_total (distance 2)
          process_payment (distance 3)
```

```
Prompt: "If validate_item changes, what is affected?"
Expected: validate_order (distance 1)
```

#### Scenario 2c: Test Coverage

```
Prompt: "What tests cover calculate_total?"
Expected: (none in current dataset — demonstrates a coverage gap)
```

**Metrics**:
- **Path completeness**: Does traversal find ALL reachable nodes within depth?
- **Path accuracy**: Are all edges in the path correctly typed (Calls vs Imports)?
- **Impact precision**: Does impact analysis correctly identify all upstream dependents?

---

### 3. Temporal Versioning

**Goal**: Verify version chains record, reconstruct, and reason about change history.

#### Scenario 3a: Version Chain Integrity

```
Action: Ingest apply_discount, then record a bug fix, then record a perf improvement.
Verify:
  - 3 versions exist for apply_discount
  - Versions are ordered by timestamp
  - Parent chain: v3 -> v2 -> v1 (v1 has no parent)
  - Each version has correct author, message, commit_id
```

#### Scenario 3b: Bug Fix Propagation

```
Action: Record a bug fix to apply_discount.
Query: "What functions need re-testing after this fix?"
Expected: calculate_total, process_payment
Reasoning: Both transitively depend on apply_discount via Calls edges.
```

#### Scenario 3c: Content Reconstruction

```
Action: Given version v2 of apply_discount, reconstruct the content at that version.
Expected: Content includes the rounding fix but not the caching optimization.
```

**Metrics**:
- **Chain correctness**: Are parent pointers accurate?
- **Reconstruction accuracy**: Does content-at-version match the expected snapshot?
- **Propagation completeness**: Are all affected nodes identified?

---

### 4. Cross-Store Fusion (Combined Queries)

**Goal**: Verify that queries combining vector search with graph expansion produce better results than either alone.

#### Scenario 4a: Semantic + Structural

```
Prompt: "payment processing"
Vector-only results: process_payment, record_transaction
Graph-expanded results: process_payment, record_transaction, calculate_total, apply_discount
Added value: Graph expansion surfaced calculate_total (called by process_payment)
```

#### Scenario 4b: Score Boosting

```
When a node appears in BOTH vector results and graph traversal, its score should be boosted.
Verify: Combined source items have higher scores than single-source items.
```

**Metrics**:
- **Expansion recall**: How many additional relevant nodes does graph expansion add?
- **Score calibration**: Do boosted scores correlate with actual relevance?

---

## How to Run Evaluations

### Automated (via tests)

```bash
cargo test
```

All 36 tests validate core scenarios:
- `test_vector_search_query` — Semantic search returns correct top result
- `test_graph_expanded_query` — Graph expansion finds transitive dependencies
- `test_impact_analysis` — Impact analysis propagates correctly
- `test_find_covering_tests` — Test coverage mapping works

### Interactive

```bash
# Run the full demo walkthrough
cargo run -- demo

# Start an interactive session with demo data
cargo run -- interactive --demo
```

In the interactive REPL, try:
```
query payment processing
traverse calculate_total --depth 3
impact apply_discount
inspect process_payment
history apply_discount
nodes
edges
```

### Ingesting Real Code

```bash
# Ingest a real Rust source file
cargo run -- interactive
> ingest path/to/your/file.rs
> nodes
> query your search terms
```

---

## Sample Prompts for AI Agent Evaluation

These prompts simulate what an AI coding agent would ask Onyx:

1. **Function discovery**: "Find all functions related to billing calculations"
2. **Dependency tracing**: "What does process_payment depend on?"
3. **Impact assessment**: "If I change the discount logic, what tests should I run?"
4. **Code understanding**: "Show me the full implementation of order validation"
5. **Change archaeology**: "What changes were made to apply_discount and why?"
6. **Coverage analysis**: "Which functions have no test coverage?"
7. **Module mapping**: "What functions are in the payment module?"
8. **Cross-cutting concerns**: "Find all functions that touch the database"

---

## Future Evaluation Improvements

- **Benchmark suite**: Synthetic codebases of varying sizes (100, 1K, 10K, 100K nodes) to measure scaling
- **Embedding comparison**: Compare BagOfWords vs transformer vs code-specific embeddings
- **HNSW vs brute-force**: Measure recall@k and latency as index size grows
- **Multi-language**: Test with Python, TypeScript, Go source files
- **Real-world datasets**: Evaluate on open-source projects (e.g., ripgrep, tokio, serde)
- **Agent integration**: Measure end-to-end task completion rate when an AI agent uses Onyx vs. raw file search
