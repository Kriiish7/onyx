# Onyx Project Roadmap

## Graph-Native Vector Memory for AI Agents

**Current Version:** v0.2.0 (Working Prototype)  
**Last Updated:** February 7, 2026

---

## Executive Summary

Onyx is a Rust-native infrastructure engine that fuses semantic vector search, structural knowledge graphs, and temporal versioning into a unified retrieval system for AI coding agents. The current v0.2.0 prototype demonstrates the core architecture with 36 passing tests, in-memory storage, and a functional CLI with interactive REPL.

This roadmap outlines the path from prototype to production-grade platform, organized into six major phases spanning from v0.3 through v2.0+.

**Target Users:** Individual developers → Small teams → Enterprises  
**Development Model:** Solo developer with open source community  
**License:** Apache 2.0 or MIT (open source through v1.0+)

---

## User And Developer Interaction Model

This section clarifies how users and developers will interact with Onyx across phases so features align with real workflows.

### Personas

- **Solo developer (CLI-first):** Runs local ingestion and queries on a repo, prefers fast setup and minimal configuration.
- **Team developer (API + UI):** Needs shared projects, access control, and a visual graph/search interface.
- **Platform engineer (ops):** Wants predictable deployment, metrics, backups, and upgrade paths.
- **Extension builder (integrations):** Uses APIs and SDKs to connect Onyx to editors, CI, or agent runtimes.

### Primary Interaction Surfaces

- **CLI + REPL:** Fast local iteration, scripted ingestion, repeatable queries, diagnostics.
- **HTTP API:** Programmatic access for agents, CI workflows, and custom tooling.
- **Web UI:** Search, graph exploration, history timeline, and admin settings.
- **Integrations:** Editor extensions, Git hooks, and webhook-driven ingestion.

### Core User Journeys

1. **Index a codebase:** Create project → ingest repo → verify nodes/edges → run first semantic query.
2. **Investigate impact:** Search symbol → traverse graph → review history → export impact report.
3. **Team collaboration:** Invite users → assign roles → share saved queries → monitor ingestion status.
4. **Operational flow:** Deploy container → configure storage → monitor metrics → upgrade with migrations.

### Developer Workflow Expectations

- **Local-first dev loop:** `cargo run -- demo` for quick iteration, feature flags for experimental paths.
- **Stable API contracts:** Versioned endpoints, backward-compatible JSON responses.
- **Extensibility:** Clear data model, predictable IDs, and documented events for integrations.
- **Observability:** Structured logs and metrics from day one to reduce support load.

### Stakeholders And Userbase

- **Individual developers:** Want fast local search and impact analysis on personal or small repos.
- **Team developers:** Need shared visibility into architecture, dependencies, and change history.
- **Tech leads and architects:** Use graph views to review structure, identify hotspots, and enforce standards.
- **Platform and DevOps engineers:** Care about deployment, monitoring, backups, and uptime.
- **Security and compliance teams:** Need audit trails, access control, and data retention policies.
- **Product engineering managers:** Track risk, change impact, and delivery velocity across codebases.
- **AI agent integrators:** Connect Onyx to agent runtimes, IDEs, and CI workflows.

---

## Solo Developer Considerations

This roadmap is designed for a solo developer while building toward enterprise scale. Key principles:

### Automation First

- **CI/CD is mandatory, not optional** - Automated testing reduces manual verification burden
- **Feature flags** - Deploy risky changes behind flags for quick rollback
- **2-week sprints** - Regular releases maintain momentum and reduce risk

### Incremental Language Support

Rather than implementing all 6 languages at once:

1. **Start with Rust** (v0.6) - you're already familiar with it
2. **Add Python** (v0.7) - largest user base
3. **Add TypeScript/JavaScript** (v0.8) - high demand
4. **Defer Go/Java** (v0.9+) - add based on user requests

### Complexity Management

- **Phase 1-2:** CLI and local usage only (no complex deployment)
- **Phase 3:** Docker/docker-compose only (K8s deferred)
- **Phase 4:** Add K8s manifests
- **Phase 5:** Full K8s with Helm
- **v2.0+:** Distributed storage (deferred from earlier phases)

### Burnout Prevention

- **MVP-first approach** - Ship working features, perfect later
- **Defer nice-to-haves** - Focus on core functionality
- **Regular breaks** - Sustainable pace beats crunch
- **Community building** - Start early to share the load

### Time-Saving Recommendations

- Use battle-tested libraries (don't reinvent)
- Buy/build tradeoff: Buy infrastructure (hosting), build core product
- Documentation as you go (not at the end)
- Test automation from day one

---

## Open Source Strategy

Onyx will remain open source through v1.0 and beyond.

### License Recommendation

**Dual licensing approach:**

- **Primary:** Apache 2.0 - Permissive, enterprise-friendly
- **Alternative:** MIT - Even more permissive if preferred
- **Avoid:** GPL - Too restrictive for enterprise adoption

### Community Building Timeline

**Phase 1 (v0.3): Foundation**

- Publish to GitHub with README
- Add LICENSE (Apache 2.0 or MIT)
- Create CONTRIBUTING.md
- Add good first issues
- Set up Discord/Slack channel

**Phase 2 (v0.4-v0.5): Engagement**

- Write blog posts about architecture
- Post on Hacker News, Reddit r/rust, r/programming
- Record demo videos
- Weekly progress updates on Twitter/Mastodon

**Phase 3 (v0.6-v0.7): Growth**

- Seek early adopters
- Respond to all issues within 24 hours
- Recognize contributors in releases
- Consider GitHub Sponsors

**Phase 4+ (v0.8+): Sustainability**

- Delegate code review to trusted contributors
- Establish governance model
- Consider Open Collective for funding
- Conference talks and podcasts

### Why Open Source Helps Solo Developers

- **Free QA** - Community finds bugs you missed
- **Free features** - Contributors add languages/integrations
- **Credibility** - Open source builds trust with enterprise customers
- **Marketing** - Technical community amplifies your work
- **Talent** - Contributors may become co-founders or employees

### Contribution Guidelines Priority

**Critical for v0.3:**

- Clear CONTRIBUTING.md
- Code style guide (rustfmt + clippy)
- Issue templates
- Pull request templates
- Code of Conduct

---

## Current State (v0.2.0)

### What Works Today

| Component              | Status     | Details                                                           |
| ---------------------- | ---------- | ----------------------------------------------------------------- |
| **Data Model**         | Complete   | Node, Edge, Version, Embedding schemas fully implemented          |
| **In-Memory Storage**  | Complete   | SurrealDB-based stores for vector, graph, and history             |
| **Persistent Storage** | Ready      | RocksDB implementation complete (1,060+ lines), needs build tools |
| **Query Engine**       | Complete   | Graph traversal, impact analysis, test coverage mapping           |
| **CLI & REPL**         | Complete   | Interactive commands, demo mode, file ingestion                   |
| **Tests**              | 36 passing | Unit tests for all subsystems                                     |

### Technical Debt

- Bag-of-words embeddings (placeholder for ONNX/transformer models)
- Brute-force kNN search (HNSW interface ready, needs implementation)
- Regex-based parsing (tree-sitter ready, needs C build tools)
- Static HTML frontend (React architecture planned)
- No authentication layer (JWT middleware ready)

---

## Phase 1: MVP Stability (v0.3)

**Duration:** 6-8 weeks  
**Target Users:** Individual developers (CLI + local usage)  
**Focus:** Local development workflow, no deployment complexity  
**Goal:** Solidify the foundation and resolve critical blockers

### Objectives

1. Enable production storage on all platforms
2. Replace placeholder implementations with production-grade alternatives
3. Establish CI/CD pipeline and quality gates
4. Create comprehensive documentation

### Deliverables

#### 1.1 Build System & Toolchain

- [ ] Cross-platform build configuration (Windows/Linux/macOS)
- [ ] Docker development environment with all native dependencies
- [ ] CI/CD pipeline (GitHub Actions): build, test, lint, security scan
- [ ] Pre-commit hooks for code quality

#### 1.2 Storage Layer Hardening

- [ ] Enable RocksDB feature on Windows (Visual Studio Build Tools documentation)
- [ ] Storage migration tooling (SurrealDB → RocksDB)
- [ ] WAL durability tests and crash recovery validation
- [ ] Benchmark suite: insert/query throughput, memory usage

#### 1.3 Testing Infrastructure

- [ ] Integration test suite for persistent stores
- [ ] Property-based tests for graph operations
- [ ] Performance regression tests
- [ ] Code coverage reporting (target: 80%+)

#### 1.4 Documentation

- [ ] API reference (rustdoc)
- [ ] Deployment guide
- [ ] Build environment setup guide
- [ ] Troubleshooting guide

### Dependencies

| Dependency                | Purpose                 | Status                    |
| ------------------------- | ----------------------- | ------------------------- |
| Visual Studio Build Tools | Windows C++ compilation | User-installed            |
| Docker                    | Development environment | Optional but recommended  |
| GitHub Actions            | CI/CD                   | Requires repository setup |

### Success Criteria

- [ ] Successful builds on Windows, Linux, and macOS
- [ ] All 36 existing tests pass + 20+ new integration tests
- [ ] RocksDB storage passes durability tests (kill -9 recovery)
- [ ] Documentation complete for all public APIs
- [ ] Docker image builds and runs successfully

### Risk Considerations

| Risk                       | Impact | Mitigation                                       |
| -------------------------- | ------ | ------------------------------------------------ |
| Windows build complexity   | High   | Provide Docker alternative, clear VS setup guide |
| RocksDB performance issues | Medium | Benchmark early, tunable parameters              |
| Test flakiness             | Medium | Property-based tests, deterministic fixtures     |

---

## Phase 2: API & Frontend (v0.4-v0.5)

**Duration:** 10-12 weeks  
**Target Users:** Individual developers & small teams (2-5 users)  
**Focus:** Single-node deployment, basic auth, simple sharing  
**Goal:** Web interface and REST API for broader accessibility

### Objectives

1. Build production-ready Axum API server
2. Implement WebSocket support for real-time collaboration
3. Create React-based web frontend
4. Add basic authentication and authorization

### Deliverables

#### 2.1 Axum API Server (v0.4)

- [ ] REST endpoints:
  - `POST /api/v1/ingest` - Code ingestion
  - `POST /api/v1/query` - Semantic search
  - `GET /api/v1/nodes/:id` - Node details
  - `GET /api/v1/nodes/:id/neighbors` - Graph traversal
  - `GET /api/v1/nodes/:id/history` - Version history
  - `POST /api/v1/traverse` - Custom traversal
  - `GET /api/v1/impact/:id` - Impact analysis
  - `GET /api/v1/tests/:id` - Test coverage
  - `GET /api/v1/stats` - Store statistics
- [ ] Middleware stack:
  - CORS configuration
  - Compression (gzip/brotli)
  - Rate limiting (token bucket)
  - Request tracing
- [ ] Error handling with structured JSON responses
- [ ] OpenAPI/Swagger specification

#### 2.2 WebSocket Support (v0.4)

- [ ] WebSocket endpoint: `/ws`
- [ ] Event protocol:
  - `subscribe` - Subscribe to project updates
  - `node_updated` - Node change notifications
  - `edge_added` - New edge notifications
  - `ingest_progress` - Ingestion progress (files_done/total)
  - `query_results` - Streaming search results
- [ ] Connection management and heartbeat

#### 2.3 Authentication (v0.4)

- [ ] JWT token generation and validation
- [ ] API key management (hashed storage)
- [ ] Basic RBAC: viewer, developer, admin roles
- [ ] Auth middleware for protected endpoints

#### 2.4 React Frontend (v0.5)

- [ ] Project setup: React + TypeScript + Vite
- [ ] State management: Zustand or Redux Toolkit
- [ ] UI component library: Tailwind CSS + Headless UI
- [ ] Core views:
  - **Graph Explorer**: D3.js/vis.js graph visualization
  - **Search Console**: Semantic search with filters
  - **Code Browser**: File tree with syntax highlighting
  - **History Timeline**: Version history visualization
  - **Settings**: API keys, project configuration
- [ ] Real-time updates via WebSocket
- [ ] Responsive design (desktop-first, tablet support)

### Dependencies

| Dependency        | Purpose      | Integration       |
| ----------------- | ------------ | ----------------- |
| axum 0.7          | HTTP server  | Core framework    |
| tokio-tungstenite | WebSocket    | Real-time updates |
| jsonwebtoken      | JWT auth     | Security          |
| tower-http        | Middleware   | CORS, compression |
| React 18          | Frontend UI  | User interface    |
| D3.js/vis-network | Graph viz    | Graph Explorer    |
| Monaco Editor     | Code display | Code Browser      |

### Success Criteria

- [ ] All REST endpoints documented and tested
- [ ] WebSocket connections stable for 24+ hours
- [ ] Frontend loads in <2 seconds (lighthouse performance)
- [ ] Authentication flow complete (login → token → API access)
- [ ] Graph visualization handles 1000+ nodes smoothly
- [ ] End-to-end tests: ingest → query → display

### Risk Considerations

| Risk                 | Impact | Mitigation                                              |
| -------------------- | ------ | ------------------------------------------------------- |
| Frontend complexity  | Medium | Start with read-only views, add mutations incrementally |
| WebSocket scaling    | Medium | Connection pooling, horizontal scaling in Phase 4       |
| Auth vulnerabilities | High   | Security audit, use proven libraries (jsonwebtoken)     |

---

## Phase 3: Production Infrastructure (v0.6-v0.7)

**Duration:** 12-16 weeks  
**Target Users:** Small teams & early enterprise adopters (5-20 users)  
**Focus:** Docker deployment, team collaboration features  
**Goal:** Production-grade embeddings, parsing, and deployment

### Objectives

1. Implement transformer-based embeddings via ONNX
2. Add tree-sitter parsing for multiple languages
3. Build HNSW vector index for scalable search
4. Create Docker and Kubernetes deployment artifacts

### Deliverables

#### 3.1 ONNX Embeddings (v0.6)

- [ ] ONNX Runtime integration (`ort` crate)
- [ ] Model: `all-MiniLM-L6-v2` (384 dimensions)
- [ ] Tokenization pipeline (BERT tokenizer)
- [ ] Batch inference (batch size: 32)
- [ ] GPU support via CUDA (optional, fallback to CPU)
- [ ] Embedding versioning (re-embed when model changes)
- [ ] Model download and caching

#### 3.2 Tree-sitter Parsing (v0.6)

- [ ] Tree-sitter core integration
- [ ] Language grammars (start with one, add incrementally):
  - **Phase 3a:** Rust (`tree-sitter-rust`) - primary focus
  - **Phase 3b:** Python (`tree-sitter-python`)
  - **Phase 3c:** TypeScript (`tree-sitter-typescript`)
  - **Phase 3d:** JavaScript (`tree-sitter-javascript`)
  - **Phase 4a:** Go (`tree-sitter-go`)
  - **Phase 4b:** Java (`tree-sitter-java`)
- [ ] AST traversal and node extraction
- [ ] Relationship detection (calls, imports, contains)
- [ ] Incremental parsing for updates
- [ ] Replace regex-based parser entirely

**Solo Developer Note:** Focus on Rust parser first (v0.6), add other languages incrementally to manage complexity.

#### 3.3 HNSW Vector Index (v0.6)

- [ ] HNSW implementation or integration
- [ ] Parameters: M=16, ef_construction=200, ef_search=100
- [ ] Persist layers in RocksDB (`hnsw_layers` CF)
- [ ] Incremental insertion (no full rebuilds)
- [ ] Concurrent search (read-only during insert)
- [ ] Benchmark: <10ms search latency for k=10

#### 3.4 Deployment Artifacts (v0.7) - Docker First

- [ ] Dockerfile (multi-stage build, <500MB)
- [ ] docker-compose.yml for local development and single-node deployment
- [ ] Environment-based configuration (12-factor app)
- [ ] Health check endpoints for container orchestration
- [ ] Volume management for persistent data
- [ ] **Note:** Kubernetes deferred to Phase 4 for complexity management

#### 3.5 Configuration Management

- [ ] TOML configuration file support
- [ ] Environment variable overrides
- [ ] Configuration validation
- [ ] Hot-reload for non-critical settings

### Dependencies

| Dependency           | Purpose      | Challenge            |
| -------------------- | ------------ | -------------------- |
| ort 2.0+             | ONNX Runtime | Large binary size    |
| tree-sitter          | AST parsing  | C build dependencies |
| hnswlib-rs or custom | Vector index | Performance tuning   |
| Docker/Kubernetes    | Deployment   | Ops expertise needed |

### Success Criteria

- [ ] ONNX embeddings achieve >0.8 correlation with semantic similarity
- [ ] Tree-sitter parses all 5 languages with 95%+ accuracy
- [ ] HNSW search 100x faster than brute-force on 100K vectors
- [ ] Docker image size <500MB
- [ ] Kubernetes deployment handles rolling updates
- [ ] Zero-downtime configuration reload

### Risk Considerations

| Risk                     | Impact | Mitigation                            |
| ------------------------ | ------ | ------------------------------------- |
| ONNX binary size         | Medium | Feature flag, lazy loading            |
| Tree-sitter build issues | High   | Docker-based dev environment          |
| HNSW complexity          | Medium | Start with library, custom impl later |
| K8s complexity           | Medium | Start with docker-compose             |

---

## Phase 4: Performance & Scale (v0.8-v0.9)

**Duration:** 10-12 weeks  
**Target Users:** Teams & growing organizations (20-100 users)  
**Focus:** Kubernetes deployment, monitoring, performance optimization  
**Goal:** Optimization, caching, and horizontal scaling

### Objectives

1. Implement query caching and result optimization
2. Add monitoring and observability
3. Enable horizontal scaling
4. Optimize for large codebases (1M+ nodes)

### Deliverables

#### 4.1 Query Optimization (v0.8)

- [ ] Query planner with cost-based optimization
- [ ] LRU cache for frequent queries (Redis optional)
- [ ] Result pagination and streaming
- [ ] Query timeout and cancellation
- [ ] Execution plan analysis

#### 4.2 Monitoring & Observability (v0.8)

- [ ] Prometheus metrics:
  - Query latency (p50, p95, p99)
  - Store size by type
  - Ingestion throughput
  - HNSW search latency
  - Transaction commit/abort rates
- [ ] Distributed tracing (OpenTelemetry)
- [ ] Structured logging (JSON format)
- [ ] Health check endpoints:
  - `/health/live` - Liveness probe
  - `/health/ready` - Readiness probe
- [ ] Grafana dashboards (optional)

#### 4.3 Caching Layer (v0.8)

- [ ] In-memory query cache (LRU)
- [ ] Distributed cache option (Redis)
- [ ] Cache invalidation strategies
- [ ] Cache warming for popular queries

#### 4.4 Kubernetes Deployment (v0.9)

- [ ] Kubernetes manifests:
  - StatefulSet (3 replicas)
  - Service (ClusterIP + LoadBalancer)
  - Ingress (NGINX with TLS)
  - PVC (100Gi SSD per replica)
  - ConfigMap (configuration)
- [ ] Rolling update strategy with zero downtime
- [ ] Resource limits and requests
- [ ] Pod disruption budgets

#### 4.5 Horizontal Scaling (v0.9)

- [ ] Read replicas for query engines
- [ ] Store partitioning strategy:
  - Vector store: embedding space sharding
  - Graph store: connected component partitioning
  - History store: entity ID range partitioning
- [ ] Load balancer configuration
- [ ] Consensus for distributed transactions (Raft/Paxos) - **Deferred to v2.0**

#### 4.6 Large Codebase Optimization (v0.9)

- [ ] Lazy loading for graph traversal
- [ ] Incremental indexing
- [ ] Background compaction jobs
- [ ] Memory-mapped file options for RocksDB
- [ ] Benchmark suite: 1M nodes, 10M edges
- [ ] **Distributed storage deferred to v2.0+** - Single-node focus through v1.0

### Dependencies

| Dependency            | Purpose             | Integration      |
| --------------------- | ------------------- | ---------------- |
| prometheus            | Metrics             | HTTP endpoint    |
| tracing-opentelemetry | Distributed tracing | Middleware       |
| redis (optional)      | Distributed cache   | Feature flag     |
| raft-rs (optional)    | Consensus           | Distributed mode |

### Success Criteria

- [ ] Query latency p99 <200ms under load
- [ ] Ingest throughput 100+ files/second
- [ ] System handles 100 concurrent users
- [ ] Cache hit rate >50% for repeated queries
- [ ] Horizontal scaling: 3x throughput with 3 replicas
- [ ] Zero data loss in failover scenarios

### Risk Considerations

| Risk                          | Impact | Mitigation                                |
| ----------------------------- | ------ | ----------------------------------------- |
| Distributed system complexity | High   | Start with single-node optimization       |
| Cache consistency             | Medium | TTL-based invalidation, short cache times |
| Monitoring overhead           | Low    | Sampling, async metric collection         |

---

## Phase 5: Enterprise Ready (v1.0)

**Duration:** 16-20 weeks  
**Target Users:** Enterprises & large organizations (100+ users, multi-tenant)  
**Focus:** Security, compliance, multi-tenancy, enterprise integrations  
**Goal:** Multi-tenancy, advanced security, and enterprise features

### Objectives

1. Implement multi-tenant architecture
2. Harden security and compliance
3. Add enterprise integrations
4. Achieve production stability

### Deliverables

#### 5.1 Multi-Tenancy (v1.0)

- [ ] Project isolation (separate stores per tenant)
- [ ] Resource quotas (storage, compute)
- [ ] Tenant-specific embeddings models
- [ ] Cross-tenant query restrictions
- [ ] Tenant onboarding/offboarding automation

#### 5.1a Helm Charts (v1.0)

- [ ] Production-ready Helm chart
- [ ] Values files for different environments (dev/staging/prod)
- [ ] Secret management integration
- [ ] Autoscaling configuration (HPA/VPA)
- [ ] Terraform modules for AWS/GCP/Azure

#### 5.2 Security Hardening (v1.0)

- [ ] Input validation and sanitization
- [ ] Resource limits (max query depth, result size, file size)
- [ ] Secrets management (Vault integration)
- [ ] Audit logging for all mutations
- [ ] SOC 2 compliance documentation
- [ ] Penetration testing

#### 5.3 Enterprise Integrations (v1.0)

- [ ] Git provider integrations:
  - GitHub (webhooks, OAuth)
  - GitLab (webhooks, OAuth)
  - Bitbucket (webhooks, OAuth)
- [ ] CI/CD integrations:
  - GitHub Actions
  - GitLab CI
  - Jenkins
- [ ] IDE integrations:
  - VS Code extension
  - JetBrains plugin
- [ ] Slack/Discord notifications

#### 5.4 Advanced Features (v1.0)

- [ ] Custom embedding models (bring your own)
- [ ] Graph analytics (centrality, clustering)
- [ ] Automated impact analysis on PR
- [ ] Semantic code review suggestions
- [ ] Export/import functionality

#### 5.5 Documentation & Support (v1.0)

- [ ] Enterprise deployment guide
- [ ] Security whitepaper
- [ ] API client libraries (Python, TypeScript)
- [ ] SLA definitions
- [ ] Support portal

### Dependencies

| Dependency       | Purpose            | Integration        |
| ---------------- | ------------------ | ------------------ |
| OAuth providers  | Git auth           | GitHub/GitLab apps |
| Vault (optional) | Secrets management | Sidecar pattern    |
| LSP protocol     | IDE integrations   | Language servers   |

### Success Criteria

- [ ] 100+ tenants running concurrently
- [ ] Zero security vulnerabilities (pen test passed)
- [ ] 99.9% uptime SLA achievable
- [ ] GitHub/GitLab integrations in production use
- [ ] SOC 2 Type II readiness
- [ ] API client libraries published to PyPI/npm

### Risk Considerations

| Risk                     | Impact   | Mitigation                       |
| ------------------------ | -------- | -------------------------------- |
| Multi-tenancy isolation  | Critical | Row-level security, separate DBs |
| Security vulnerabilities | Critical | Regular audits, bug bounty       |
| Integration maintenance  | Medium   | Abstract provider interfaces     |

---

## Phase 6: Platform Scale (v2.0+)

**Duration:** Ongoing / SaaS transition  
**Target Users:** Enterprise SaaS customers (managed service)  
**Focus:** Distributed systems, managed offering, plugin ecosystem  
**Goal:** Distributed systems, SaaS platform, and ecosystem

### Objectives

1. Build managed SaaS offering
2. Implement true distributed storage
3. Create plugin ecosystem
4. Expand to general knowledge graphs (beyond code)

### Deliverables

#### 6.1 SaaS Platform (v2.0)

- [ ] Multi-region deployment
- [ ] Automatic scaling (KEDA/HPA)
- [ ] Usage-based billing
- [ ] Self-service onboarding
- [ ] Admin dashboard for tenant management

#### 6.2 Distributed Storage (v2.0)

- [ ] Vector store: distributed HNSW (sharded by partition)
- [ ] Graph store: distributed adjacency lists
- [ ] Consensus protocol for writes
- [ ] Cross-shard query routing
- [ ] Geographic replication

#### 6.3 Plugin Ecosystem (v2.1)

- [ ] Plugin API (WASM-based)
- [ ] Marketplace for community plugins
- [ ] Custom analyzers
- [ ] Custom visualizations
- [ ] Integration plugins

#### 6.4 Knowledge Graph Expansion (v2.2)

- [ ] General document ingestion (PDF, Word)
- [ ] Multi-modal embeddings (images, diagrams)
- [ ] Cross-domain graph linking
- [ ] Natural language to graph queries

#### 6.5 AI Agent Integration (v2.3)

- [ ] MCP (Model Context Protocol) server
- [ ] LangChain/LlamaIndex integrations
- [ ] Autonomous agent workflows
- [ ] Reasoning engine over knowledge graphs

### Dependencies

| Dependency            | Purpose             | Timeline |
| --------------------- | ------------------- | -------- |
| Kubernetes operators  | SaaS management     | v2.0     |
| WASM runtime          | Plugin system       | v2.1     |
| Distributed consensus | Distributed storage | v2.0     |

### Success Criteria

- [ ] 1000+ active SaaS customers
- [ ] 99.99% uptime
- [ ] Plugin marketplace with 50+ plugins
- [ ] Multi-modal knowledge graphs in production
- [ ] AI agents autonomously using Onyx

---

## Technical Debt Management

### Current Debt (v0.2.0)

| Item                    | Severity | Resolution Phase |
| ----------------------- | -------- | ---------------- |
| Bag-of-words embeddings | High     | Phase 3 (v0.6)   |
| Brute-force kNN         | High     | Phase 3 (v0.6)   |
| Regex-based parsing     | High     | Phase 3 (v0.6)   |
| Static HTML frontend    | Medium   | Phase 2 (v0.5)   |
| No authentication       | Medium   | Phase 2 (v0.4)   |
| No persistent storage   | High     | Phase 1 (v0.3)   |

### Debt Prevention

1. **Code Quality for Solo Dev**
   - Self-review with 24hr delay (fresh eyes)
   - No TODOs without GitHub issues
   - Architecture Decision Records (ADRs) for major changes
   - Automated linting (clippy, rustfmt) - zero tolerance

2. **Testing Requirements**
   - Unit tests for all new code
   - Integration tests for store operations
   - Performance tests for query engine
   - **Rule:** No PR without tests (even solo)

3. **Documentation Requirements**
   - Public APIs must be documented
   - Complex algorithms need design docs
   - Breaking changes need migration guides
   - Document as you go (not at the end)

---

## Risk Mitigation for Solo Developer

### MVP-First Approach

- **Ship early, ship often** - Get feedback before over-engineering
- **YAGNI** (You Aren't Gonna Need It) - Don't build features on speculation
- **Vertical slices** - Complete one feature end-to-end before starting the next
- **Working > perfect** - Functional code beats elegant but incomplete code

### Feature Flags for Risky Changes

Implement feature flags from Phase 1:

```rust
if config.features.new_parser_enabled {
    tree_sitter_parse(source)
} else {
    regex_parse(source)
}
```

Benefits:

- Deploy incomplete features safely
- Quick rollback without reverting commits
- A/B test new features with select users
- Reduce deployment anxiety

### Regular Release Cadence

**2-week sprints with weekly checkpoints:**

- Week 1: Feature development
- Week 2: Polish, test, release
- No feature work in release week

Benefits:

- Maintains momentum
- Smaller, safer changes
- Regular user feedback
- Easier to course-correct

### Burnout Prevention Strategies

1. **Scope ruthlessly** - Cut features to maintain timeline
2. **Automate everything** - CI/CD, testing, deployment
3. **Document as you go** - Don't save it all for the end
4. **Take real weekends** - Sustainable pace > heroics
5. **Outsource non-core** - Use SaaS for auth, hosting, monitoring
6. **Celebrate milestones** - Acknowledge progress publicly
7. **Have an exit strategy** - Know when to hire/seek funding

### Technical Risk Mitigation

- **Use boring technology** - Proven tools over cutting-edge
- **Avoid distributed systems** - Until v2.0+ (deferred from Phase 4)
- **Test in production** - Feature flags enable safe testing
- **Keep it simple** - Complexity is the enemy of solo developers
- **Monitor everything** - You can't fix what you can't see

---

## Testing Strategy Evolution

### Phase 1 (v0.3): Foundation

- Unit tests for all modules
- Property-based tests for graph operations
- Storage durability tests (crash recovery)
- Benchmark baseline establishment

### Phase 2 (v0.4-v0.5): Integration

- API contract tests
- End-to-end tests (frontend → API → storage)
- WebSocket load tests
- Authentication flow tests

### Phase 3 (v0.6-v0.7): Performance

- Embedding quality tests
- HNSW accuracy tests
- Parse accuracy tests (tree-sitter)
- Load tests: 100 concurrent users

### Phase 4 (v0.8-v0.9): Scale

- Horizontal scaling tests
- Failover tests
- Cache consistency tests
- Large codebase tests (1M+ nodes)

### Phase 5+ (v1.0+): Enterprise

- Multi-tenancy isolation tests
- Security penetration tests
- Integration tests (GitHub, GitLab)
- Chaos engineering tests

---

## Documentation Requirements

### Phase 1 (v0.3)

- [ ] API reference (rustdoc)
- [ ] Build environment setup
- [ ] Deployment guide (Docker)
- [ ] Troubleshooting guide

### Phase 2 (v0.4-v0.5)

- [ ] REST API specification (OpenAPI)
- [ ] WebSocket protocol documentation
- [ ] Frontend architecture guide
- [ ] Authentication guide

### Phase 3 (v0.6-v0.7)

- [ ] Embedding model guide
- [ ] Language support matrix (see below)
- [ ] Docker deployment guide
- [ ] Configuration reference

#### Language Support Matrix (Phase 3)

| Language   | Parser                 | Status          | Target Phase |
| ---------- | ---------------------- | --------------- | ------------ |
| Rust       | tree-sitter-rust       | Primary focus   | v0.6         |
| Python     | tree-sitter-python     | High priority   | v0.7         |
| TypeScript | tree-sitter-typescript | High priority   | v0.8         |
| JavaScript | tree-sitter-javascript | High priority   | v0.8         |
| Go         | tree-sitter-go         | Medium priority | v0.9         |
| Java       | tree-sitter-java       | Medium priority | v0.9         |

### Phase 4 (v0.8-v0.9)

- [ ] Performance tuning guide
- [ ] Monitoring setup guide
- [ ] Scaling guide
- [ ] Troubleshooting at scale

### Phase 5+ (v1.0+)

- [ ] Enterprise deployment guide
- [ ] Security whitepaper
- [ ] Integration guides (GitHub, VS Code)
- [ ] API client documentation

---

## Community & Contributor Considerations

### Phase 1 (v0.3) - Foundation

- [ ] CONTRIBUTING.md with clear guidelines
- [ ] Code of Conduct (Citizen Code of Conduct template)
- [ ] Good first issues labeled (at least 5)
- [ ] Development setup documentation
- [ ] LICENSE file (Apache 2.0 or MIT)
- [ ] README with quickstart

### Phase 2 (v0.4-v0.5) - Engagement

- [ ] Discord or Slack community
- [ ] **Bi-weekly** updates (not monthly - solo dev reality)
- [ ] Contributor recognition in releases/CHANGELOG
- [ ] GitHub Discussions for RFCs (lightweight process)
- [ ] Twitter/Mastodon presence

### Phase 3 (v0.6-v0.7) - Growth

- [ ] Respond to issues within 24 hours
- [ ] Blog posts on technical decisions
- [ ] Conference talks (one per quarter max to avoid burnout)
- [ ] GitHub Sponsors or Open Collective

### Phase 4+ (v0.8+) - Sustainability

- [ ] Delegate to trusted maintainers
- [ ] Simple governance model (BDFL model works for solo)
- [ ] Consider incorporation if revenue warrants it

**Solo Developer Note:** Community building is marketing. Start early but keep time investment sustainable.

---

## Migration Paths Between Versions

### v0.2.0 → v0.3.0

- **Breaking Changes:** None (additive only)
- **Migration:** Optional switch from SurrealDB to RocksDB
- **Tooling:** `onyx migrate` command
- **Rollback:** Keep SurrealDB stores, dual-write during transition

### v0.3.0 → v0.4.0

- **Breaking Changes:** API path changes, auth required
- **Migration:** Update client code to use new endpoints
- **Tooling:** API versioning (`/api/v1/`)
- **Rollback:** Keep v0.3 endpoints as deprecated for 1 release

### v0.5.0 → v0.6.0

- **Breaking Changes:** Embedding format change (BagOfWords → ONNX)
- **Migration:** Re-ingest all code after deployment
- **Tooling:** `onyx reingest --all` command
- **Rollback:** Not recommended (embeddings incompatible)

### v0.x → v1.0.0

- **Breaking Changes:** Multi-tenancy model, API changes
- **Migration:** Project migration tool
- **Tooling:** Automated migration scripts
- **Rollback:** Backup and restore process documented

---

## Appendix: Technology Stack Summary

### Core Technologies

| Layer         | Technology      | Version |
| ------------- | --------------- | ------- |
| Language      | Rust            | 1.75+   |
| Async Runtime | Tokio           | 1.x     |
| Serialization | serde + bincode | Latest  |
| Database      | RocksDB         | 0.22+   |

### API & Frontend

| Layer       | Technology            | Version |
| ----------- | --------------------- | ------- |
| HTTP Server | Axum                  | 0.7+    |
| WebSocket   | tokio-tungstenite     | 0.24+   |
| Auth        | jsonwebtoken + bcrypt | Latest  |
| Frontend    | React + TypeScript    | 18+     |
| Styling     | Tailwind CSS          | 3.x     |

### ML & Parsing

| Layer        | Technology           | Version |
| ------------ | -------------------- | ------- |
| Embeddings   | ONNX Runtime (ort)   | 2.0+    |
| Model        | all-MiniLM-L6-v2     | -       |
| Parsing      | tree-sitter          | 0.23+   |
| Vector Index | HNSW (custom or lib) | -       |

### Language Support

| Language   | Parser                 | Priority | Phase |
| ---------- | ---------------------- | -------- | ----- |
| Rust       | tree-sitter-rust       | P0       | v0.6  |
| Python     | tree-sitter-python     | P0       | v0.7  |
| TypeScript | tree-sitter-typescript | P1       | v0.8  |
| JavaScript | tree-sitter-javascript | P1       | v0.8  |
| Go         | tree-sitter-go         | P2       | v0.9  |
| Java       | tree-sitter-java       | P2       | v0.9  |

### Infrastructure

| Layer         | Technology    | Version | Availability |
| ------------- | ------------- | ------- | ------------ |
| Container     | Docker        | 24+     | Phase 3+     |
| Orchestration | Kubernetes    | 1.28+   | Phase 4+     |
| Monitoring    | Prometheus    | Latest  | Phase 4+     |
| Tracing       | OpenTelemetry | Latest  | Phase 4+     |

---

## Appendix: Performance Targets

| Metric               | Target          | Phase   |
| -------------------- | --------------- | ------- |
| Query latency (p50)  | <50ms           | Phase 4 |
| Query latency (p99)  | <200ms          | Phase 4 |
| Ingest throughput    | 100 files/sec   | Phase 4 |
| HNSW search (k=10)   | <10ms           | Phase 3 |
| Concurrent users     | 100             | Phase 4 |
| Storage efficiency   | <1MB per 1K LOC | Phase 4 |
| Uptime               | 99.9%           | Phase 5 |
| Tenants (concurrent) | 100+            | Phase 5 |

---

## Contributing to This Roadmap

This roadmap is a living document. To propose changes:

1. Open an issue with the `roadmap` label
2. Discuss in community channels
3. Submit PR with rationale

Major changes require approval from maintainers.

---

## License

This roadmap is part of the Onyx project. See LICENSE.txt for details.
