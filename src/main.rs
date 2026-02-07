use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use onyx::error::OnyxResult;
use onyx::ingest::{ingest_codebase, parse_rust_source, CodeUnit};
use onyx::model::edge::EdgeType;
use onyx::model::embedding::BagOfWordsEmbedder;
use onyx::model::node::NodeType;
use onyx::query::{execute_query, find_covering_tests, impact_analysis, QueryOptions};
use onyx::store::benchmark::BenchmarkRunner;
use onyx::store::crash_recovery::CrashTestRunner;
use onyx::store::graph::GraphStore;
use onyx::store::history::HistoryStore;
use onyx::store::migration::run_migration;
use onyx::store::transaction::TransactionManager;

/// Onyx: Graph-Native Vector Memory for AI Agents
#[derive(Parser)]
#[command(name = "onyx")]
#[command(
    about = "Graph-native vector memory for AI agents. Fuses semantic search, knowledge graphs, and temporal versioning."
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest code artifacts from a file or directory
    Ingest {
        /// Path to a Rust source file or directory
        #[arg(short, long)]
        path: PathBuf,
    },
    /// Run a semantic query against the store
    Query {
        /// The query string
        #[arg(short, long)]
        question: String,
        /// Maximum traversal depth
        #[arg(short, long, default_value = "2")]
        depth: usize,
        /// Number of vector search results
        #[arg(short, long, default_value = "5")]
        top_k: usize,
    },
    /// Traverse the graph from a node
    Traverse {
        /// Node name to start from
        #[arg(short, long)]
        node: String,
        /// Edge types to follow (comma-separated: calls,imports,defines,contains,tests,depends)
        #[arg(short, long)]
        relations: Option<String>,
        /// Maximum depth
        #[arg(short, long, default_value = "2")]
        depth: usize,
    },
    /// Inspect a specific node
    Inspect {
        /// Node name to inspect
        #[arg(short, long)]
        node: String,
    },
    /// Show store statistics
    Status,
    /// Run a demo with a synthetic codebase
    Demo,
    /// Start an interactive REPL session
    Interactive {
        /// Pre-load the demo dataset on startup
        #[arg(long)]
        demo: bool,
    },
    /// Migrate data between storage backends
    Migrate {
        /// Target storage path for RocksDB
        #[arg(short, long)]
        path: PathBuf,
    },
    /// Test crash recovery and WAL durability
    TestCrashRecovery {
        /// Database path for testing
        #[arg(short, long)]
        path: PathBuf,
    },
    /// Run performance benchmarks
    Benchmark {
        /// Database path for testing
        #[arg(short, long)]
        path: PathBuf,
        /// Number of operations
        #[arg(long, default_value = "10000")]
        operations: usize,
        /// Concurrency level
        #[arg(long, default_value = "10")]
        concurrency: usize,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Demo => {
            if let Err(e) = run_demo().await {
                eprintln!("Demo failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Interactive { demo } => {
            if let Err(e) = run_interactive(demo).await {
                eprintln!("Interactive session failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Status => {
            println!("Onyx store is empty (no persistent storage).");
            println!("Use 'onyx interactive' for a REPL session with shared in-memory stores.");
            println!("Use 'onyx interactive --demo' to pre-load the demo dataset.");
            println!("Use 'onyx demo' for a non-interactive demo walkthrough.");
        }
        Commands::Ingest { path } => {
            println!("Ingesting from: {}", path.display());
            if let Err(e) = run_ingest(&path).await {
                eprintln!("Ingestion failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Query {
            question,
            depth,
            top_k,
        } => {
            println!("Query: '{}' (depth={}, top_k={})", question, depth, top_k);
            println!("Tip: Use 'onyx interactive --demo' for a session with pre-loaded data.");
        }
        Commands::Traverse {
            node,
            relations,
            depth,
        } => {
            println!(
                "Traverse from '{}' (relations={:?}, depth={})",
                node, relations, depth
            );
            println!("Tip: Use 'onyx interactive --demo' for a session with pre-loaded data.");
        }
        Commands::Inspect { node } => {
            println!("Inspect node: '{}'", node);
            println!("Tip: Use 'onyx interactive --demo' for a session with pre-loaded data.");
        }
        Commands::Migrate { path } => {
            println!("Migrating data to RocksDB at: {}", path.display());
            if let Err(e) = run_migration(&path.to_string_lossy()).await {
                eprintln!("Migration failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::TestCrashRecovery { path } => {
            println!("Running crash recovery tests at: {}", path.display());
            let mut runner = CrashTestRunner::new(&path);
            
            match runner.run_test_suite().await {
                Ok(results) => {
                    println!("\n=== Crash Recovery Test Results ===");
                    for result in &results {
                        result.print_report();
                    }
                    
                    let passed = results.iter().filter(|r| r.passed).count();
                    let total = results.len();
                    
                    println!("\n=== Summary ===");
                    println!("Passed: {}/{}", passed, total);
                    println!("Success Rate: {:.1}%", (passed as f64 / total as f64) * 100.0);
                    
                    if passed == total {
                        println!("✓ All crash recovery tests passed!");
                    } else {
                        println!("✗ Some crash recovery tests failed!");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Crash recovery test failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Benchmark { path, operations, concurrency } => {
            println!("Running performance benchmarks...");
            println!("Database path: {}", path.display());
            println!("Operations: {}", operations);
            println!("Concurrency: {}", concurrency);
            
            let config = onyx::store::benchmark::BenchmarkConfig {
                operation_count: operations,
                concurrency,
                warmup_count: operations / 10,
                measure_memory: true,
                detailed_latency: true,
            };
            
            let runner = BenchmarkRunner::new(config, &path);
            
            match runner.run_all_benchmarks().await {
                Ok(results) => {
                    println!("\n=== Benchmark Complete ===");
                    
                    // Check if we meet performance targets
                    let node_insert_ops = results.get("node_insert").map(|r| r.ops_per_second).unwrap_or(0.0);
                    let vector_search_ops = results.get("vector_search").map(|r| r.ops_per_second).unwrap_or(0.0);
                    let graph_traversal_ops = results.get("graph_traversal").map(|r| r.ops_per_second).unwrap_or(0.0);
                    
                    let targets_met = node_insert_ops > 1000.0 && vector_search_ops > 500.0 && graph_traversal_ops > 200.0;
                    
                    if targets_met {
                        println!("✓ All performance targets met!");
                    } else {
                        println!("⚠ Some performance targets not met:");
                        if node_insert_ops <= 1000.0 {
                            println!("  - Node insertion: {:.1} (target: >1000)", node_insert_ops);
                        }
                        if vector_search_ops <= 500.0 {
                            println!("  - Vector search: {:.1} (target: >500)", vector_search_ops);
                        }
                        if graph_traversal_ops <= 200.0 {
                            println!("  - Graph traversal: {:.1} (target: >200)", graph_traversal_ops);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Benchmark failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Interactive REPL
// ---------------------------------------------------------------------------

/// Shared session state for the REPL.
struct Session {
    stores: TransactionManager,
    embedder: Option<BagOfWordsEmbedder>,
}

impl Session {
    fn new() -> Self {
        Self {
            stores: TransactionManager::new(),
            embedder: None,
        }
    }

    /// Rebuild the embedder from all node contents currently in the graph store.
    #[allow(dead_code)]
    async fn rebuild_embedder(&mut self) {
        let all_nodes = self.stores.graph_store.all_nodes().await;
        if all_nodes.is_empty() {
            self.embedder = None;
            return;
        }
        let corpus: Vec<&str> = all_nodes.iter().map(|n| n.content.as_str()).collect();
        self.embedder = Some(BagOfWordsEmbedder::from_corpus(&corpus, 100));
    }
}

async fn run_interactive(load_demo: bool) -> OnyxResult<()> {
    let mut session = Session::new();

    println!("=== Onyx Interactive REPL ===");
    println!("Graph-native vector memory for AI agents.\n");

    if load_demo {
        load_demo_data(&mut session).await?;
    } else {
        println!("Store is empty. Commands:");
    }

    print_help();

    let stdin = io::stdin();
    loop {
        print!("\nonyx> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();
        let cmd = parts[0].to_lowercase();
        let args = if parts.len() > 1 { parts[1].trim() } else { "" };

        match cmd.as_str() {
            "help" | "h" | "?" => print_help(),
            "quit" | "exit" | "q" => {
                println!("Goodbye.");
                break;
            }
            "status" | "stats" => cmd_status(&session),
            "load-demo" => {
                if let Err(e) = load_demo_data(&mut session).await {
                    eprintln!("  Error: {}", e);
                }
            }
            "ingest" => {
                if args.is_empty() {
                    println!("  Usage: ingest <path-to-rust-file>");
                } else {
                    if let Err(e) = cmd_ingest(&mut session, args).await {
                        eprintln!("  Error: {}", e);
                    }
                }
            }
            "query" | "search" => {
                if args.is_empty() {
                    println!("  Usage: query <search terms> [--depth N] [--top-k N]");
                } else {
                    if let Err(e) = cmd_query(&session, args).await {
                        eprintln!("  Error: {}", e);
                    }
                }
            }
            "traverse" | "walk" => {
                if args.is_empty() {
                    println!(
                        "  Usage: traverse <node-name> [--depth N] [--relations calls,imports,...]"
                    );
                } else {
                    if let Err(e) = cmd_traverse(&session, args).await {
                        eprintln!("  Error: {}", e);
                    }
                }
            }
            "inspect" | "show" => {
                if args.is_empty() {
                    println!("  Usage: inspect <node-name>");
                } else {
                    if let Err(e) = cmd_inspect(&session, args).await {
                        eprintln!("  Error: {}", e);
                    }
                }
            }
            "impact" => {
                if args.is_empty() {
                    println!("  Usage: impact <node-name> [--depth N]");
                } else {
                    if let Err(e) = cmd_impact(&session, args).await {
                        eprintln!("  Error: {}", e);
                    }
                }
            }
            "tests" => {
                if args.is_empty() {
                    println!("  Usage: tests <node-name>");
                } else {
                    if let Err(e) = cmd_tests(&session, args).await {
                        eprintln!("  Error: {}", e);
                    }
                }
            }
            "nodes" | "list" => {
                cmd_list_nodes(&session, args);
            }
            "edges" => {
                cmd_list_edges(&session).await;
            }
            "history" => {
                if args.is_empty() {
                    println!("  Usage: history <node-name>");
                } else {
                    if let Err(e) = cmd_history(&session, args).await {
                        eprintln!("  Error: {}", e);
                    }
                }
            }
            _ => {
                println!(
                    "  Unknown command: '{}'. Type 'help' for available commands.",
                    cmd
                );
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("  Commands:");
    println!("    status              Show store statistics");
    println!("    load-demo           Load the synthetic e-commerce demo dataset");
    println!("    ingest <path>       Ingest a Rust source file");
    println!("    query <terms>       Semantic search (e.g. 'query payment processing')");
    println!(
        "    traverse <name>     Walk the graph from a node (e.g. 'traverse calculate_total')"
    );
    println!("    inspect <name>      Show full details for a node");
    println!("    impact <name>       Impact analysis: what is affected if this node changes?");
    println!("    tests <name>        Find tests covering a node");
    println!(
        "    nodes [type]        List all nodes (optionally filter by type: code/doc/test/config)"
    );
    println!("    edges               List all edges in the graph");
    println!("    history <name>      Show version history for a node");
    println!("    help                Show this help message");
    println!("    quit                Exit the REPL");
}

// ---------------------------------------------------------------------------
// REPL commands
// ---------------------------------------------------------------------------

fn cmd_status(session: &Session) {
    let stats = session.stores.stats();
    println!("  {}", stats);
    if session.embedder.is_some() {
        println!("  Embedder: active (bag-of-words, dim=100)");
    } else {
        println!("  Embedder: not initialized (ingest data to build)");
    }
}

async fn load_demo_data(session: &mut Session) -> OnyxResult<()> {
    let units = build_synthetic_codebase();
    let corpus: Vec<&str> = units.iter().map(|u| u.content.as_str()).collect();
    let embedder = BagOfWordsEmbedder::from_corpus(&corpus, 100);

    println!("  Loading {} demo artifacts...", units.len());
    let results = ingest_codebase(&mut session.stores, &units, &embedder).await?;

    for result in &results {
        let node = session
            .stores
            .graph_store
            .get_node(&result.node_id)
            .await?
            .unwrap();
        println!("    + {} ({})", node.name, result.version_id);
    }

    let stats = session.stores.stats();
    println!("  Done. {}", stats);

    // Rebuild embedder with all content
    session.embedder = Some(embedder);
    Ok(())
}

async fn cmd_ingest(session: &mut Session, path_str: &str) -> OnyxResult<()> {
    let path = PathBuf::from(path_str);
    if !path.exists() {
        println!("  File not found: {}", path.display());
        return Ok(());
    }

    let source = std::fs::read_to_string(&path)?;
    let units = parse_rust_source(&source, &path.to_string_lossy());

    if units.is_empty() {
        println!("  No code entities found in {}", path.display());
        return Ok(());
    }

    println!("  Parsed {} code entities:", units.len());
    for unit in &units {
        println!("    - {} ({:?})", unit.name, unit.kind);
    }

    // Build embedder from current corpus + new content
    let all_nodes = session.stores.graph_store.all_nodes().await;
    let mut corpus: Vec<String> = all_nodes.iter().map(|n| n.content.clone()).collect();
    for unit in &units {
        corpus.push(unit.content.clone());
    }
    let corpus_refs: Vec<&str> = corpus.iter().map(|s| s.as_str()).collect();
    let embedder = BagOfWordsEmbedder::from_corpus(&corpus_refs, 100);

    let results = ingest_codebase(&mut session.stores, &units, &embedder).await?;

    println!("  Ingested {} nodes", results.len());
    let stats = session.stores.stats();
    println!("  {}", stats);

    // Update embedder
    session.embedder = Some(embedder);

    Ok(())
}

async fn cmd_query(session: &Session, args: &str) -> OnyxResult<()> {
    let embedder = match &session.embedder {
        Some(e) => e,
        None => {
            println!("  No data loaded. Use 'load-demo' or 'ingest <path>' first.");
            return Ok(());
        }
    };

    // Parse optional flags
    let mut terms = args.to_string();
    let mut depth: usize = 2;
    let mut top_k: usize = 5;

    if let Some(idx) = terms.find("--depth") {
        let rest = &terms[idx + 7..].trim_start();
        if let Some(val) = rest.split_whitespace().next() {
            depth = val.parse().unwrap_or(2);
        }
        terms = terms[..idx].to_string();
    }
    if let Some(idx) = terms.find("--top-k") {
        let rest = &terms[idx + 7..].trim_start();
        if let Some(val) = rest.split_whitespace().next() {
            top_k = val.parse().unwrap_or(5);
        }
        terms = terms[..idx].to_string();
    }
    let terms = terms.trim();

    let query_embedding = embedder.embed(terms);
    let options = QueryOptions {
        top_k,
        max_depth: depth,
        edge_types: Some(vec![EdgeType::Calls, EdgeType::Imports, EdgeType::Contains]),
        include_history: true,
        ..Default::default()
    };

    let result = execute_query(&session.stores, Some(&query_embedding.values), &options).await?;

    println!(
        "  Found {} results ({} nodes examined, {}ms):\n",
        result.items.len(),
        result.nodes_examined,
        result.query_time_ms
    );

    for (i, item) in result.items.iter().enumerate() {
        println!(
            "  {}. [{:.3}] {} (depth {}, source: {:?})",
            i + 1,
            item.score,
            item.name,
            item.depth,
            item.source
        );
        // Show first line of content
        let first_line = item.content.lines().next().unwrap_or("");
        println!("     {}", first_line);
        for v in &item.versions {
            println!(
                "     v{}: {} ({} lines changed)",
                &v.version_id[..v.version_id.len().min(12)],
                v.message.as_deref().unwrap_or("no message"),
                v.lines_changed
            );
        }
    }

    Ok(())
}

async fn cmd_traverse(session: &Session, args: &str) -> OnyxResult<()> {
    // Parse: <node-name> [--depth N] [--relations calls,imports,...]
    let mut name = args.to_string();
    let mut depth: usize = 2;
    let mut edge_types: Option<Vec<EdgeType>> = None;

    if let Some(idx) = name.find("--depth") {
        let rest = &name[idx + 7..].trim_start();
        if let Some(val) = rest.split_whitespace().next() {
            depth = val.parse().unwrap_or(2);
        }
        name = name[..idx].to_string();
    }
    if let Some(idx) = name.find("--relations") {
        let rest = &name[idx + 11..].trim_start();
        if let Some(val) = rest.split_whitespace().next() {
            edge_types = Some(parse_edge_types(val));
        }
        name = name[..idx].to_string();
    }
    let name = name.trim();

    let node = find_node_by_name(&session.stores, name).await;
    let node = match node {
        Some(n) => n,
        None => {
            println!(
                "  Node '{}' not found. Use 'nodes' to list available nodes.",
                name
            );
            return Ok(());
        }
    };

    let node_id = node.id;
    let node_name = node.name.clone();

    println!("  Traversal from '{}' (depth {}):\n", node_name, depth);

    let traversal = session
        .stores
        .graph_store
        .traverse(&node_id, edge_types.as_deref(), depth)
        .await?;

    for (nid, d) in &traversal.nodes {
        if let Some(n) = session.stores.graph_store.get_node(nid).await? {
            let indent = "  ".repeat(*d + 1);
            let marker = if *d == 0 { "*" } else { "-" };
            println!("  {}{} {} (depth {})", indent, marker, n.name, d);
        }
    }

    // Show inbound callers
    let callers = session
        .stores
        .graph_store
        .get_inbound(&node_id, edge_types.as_deref())
        .await?;
    if !callers.is_empty() {
        println!("\n  Inbound references to '{}':", node_name);
        for (edge, caller) in &callers {
            println!("    <- {} ({:?})", caller.name, edge.edge_type);
        }
    }

    Ok(())
}

async fn cmd_inspect(session: &Session, args: &str) -> OnyxResult<()> {
    let name = args.trim();
    let node = match find_node_by_name(&session.stores, name).await {
        Some(n) => n,
        None => {
            println!(
                "  Node '{}' not found. Use 'nodes' to list available nodes.",
                name
            );
            return Ok(());
        }
    };

    println!("  === {} ===", node.name);
    println!("  ID:      {}", node.id);
    println!("  Type:    {:?}", node.node_type);
    println!(
        "  File:    {}",
        node.provenance.file_path.as_deref().unwrap_or("(unknown)")
    );
    if let Some((start, end)) = node.provenance.line_range {
        println!("  Lines:   {}-{}", start, end);
    }
    if let Some(ref commit) = node.provenance.commit_id {
        println!("  Commit:  {}", commit);
    }
    if let Some(ref branch) = node.provenance.branch {
        println!("  Branch:  {}", branch);
    }
    println!("  Created: {}", node.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("  Updated: {}", node.updated_at.format("%Y-%m-%d %H:%M:%S"));

    // Extension info
    match &node.extension {
        onyx::model::node::NodeExtension::CodeEntity(ext) => {
            println!("  Lang:    {:?}", ext.language);
            println!("  Vis:     {:?}", ext.visibility);
            if let Some(ref sig) = ext.signature {
                println!("  Sig:     {}", sig);
            }
            if !ext.module_path.is_empty() {
                println!("  Module:  {}", ext.module_path.join("::"));
            }
        }
        _ => {}
    }

    // Content
    println!("\n  --- Content ---");
    for line in node.content.lines() {
        println!("  | {}", line);
    }

    // Edges out
    let neighbors = session.stores.graph_store.get_neighbors(&node.id, None).await?;
    if !neighbors.is_empty() {
        println!("\n  --- Outbound Edges ---");
        for (edge, target) in &neighbors {
            println!(
                "    -> {} ({:?}, conf: {:.2})",
                target.name, edge.edge_type, edge.confidence
            );
        }
    }

    // Edges in
    let inbound = session.stores.graph_store.get_inbound(&node.id, None).await?;
    if !inbound.is_empty() {
        println!("\n  --- Inbound Edges ---");
        for (edge, source) in &inbound {
            println!(
                "    <- {} ({:?}, conf: {:.2})",
                source.name, edge.edge_type, edge.confidence
            );
        }
    }

    // Version history
    let versions = session.stores.history_store.list_versions(&node.id).await?;
    if !versions.is_empty() {
        println!("\n  --- Version History ---");
        for v in &versions {
            println!(
                "    {} | {} | {} | {} lines changed",
                &v.version_id[..v.version_id.len().min(12)],
                v.timestamp.format("%Y-%m-%d %H:%M:%S"),
                v.message.as_deref().unwrap_or("(no message)"),
                v.diff.lines_changed()
            );
        }
    }

    // Embedding
    if let Some(ref emb) = node.embedding {
        println!(
            "\n  Embedding: {} dimensions (first 5: {:?}...)",
            emb.len(),
            &emb[..emb.len().min(5)]
        );
    }

    Ok(())
}

async fn cmd_impact(session: &Session, args: &str) -> OnyxResult<()> {
    let mut name = args.to_string();
    let mut depth: usize = 3;

    if let Some(idx) = name.find("--depth") {
        let rest = &name[idx + 7..].trim_start();
        if let Some(val) = rest.split_whitespace().next() {
            depth = val.parse().unwrap_or(3);
        }
        name = name[..idx].to_string();
    }
    let name = name.trim();

    let node = match find_node_by_name(&session.stores, name).await {
        Some(n) => n,
        None => {
            println!("  Node '{}' not found.", name);
            return Ok(());
        }
    };

    let affected = impact_analysis(&session.stores, &node.id, depth).await?;

    println!("  Impact analysis for '{}' (depth {}):\n", node.name, depth);

    if affected.is_empty() {
        println!("  No downstream impact detected.");
    } else {
        for (_, aff_name, dist) in &affected {
            let bar = ">".repeat(*dist);
            println!("  {} {} (distance {})", bar, aff_name, dist);
        }
    }

    Ok(())
}

async fn cmd_tests(session: &Session, args: &str) -> OnyxResult<()> {
    let name = args.trim();
    let node = match find_node_by_name(&session.stores, name).await {
        Some(n) => n,
        None => {
            println!("  Node '{}' not found.", name);
            return Ok(());
        }
    };

    let tests = find_covering_tests(&session.stores, &node.id, 2).await?;

    println!("  Tests covering '{}':\n", node.name);

    if tests.is_empty() {
        println!("  (no tests found)");
    } else {
        for t in &tests {
            println!("  - {} (score: {:.2}, depth: {})", t.name, t.score, t.depth);
        }
    }

    Ok(())
}

fn cmd_list_nodes(session: &Session, filter: &str) {
    // Note: This should be async but we're keeping it simple for now
    println!("  (async node listing not yet implemented)");
}

async fn cmd_list_edges(session: &Session) {
    let edge_types = [
        EdgeType::Calls,
        EdgeType::Imports,
        EdgeType::Defines,
        EdgeType::Contains,
        EdgeType::TestsOf,
        EdgeType::Documents,
        EdgeType::DependsOn,
        EdgeType::Implements,
        EdgeType::Configures,
        EdgeType::VersionedBy,
    ];

    let mut total = 0;
    for et in &edge_types {
        let edges = session.stores.graph_store.edges_by_type(et).await;
        if !edges.is_empty() {
            if total == 0 {
                println!("  Edges in the graph:\n");
            }
            for edge in &edges {
                let source_name = session
                    .stores
                    .graph_store
                    .get_node(&edge.source_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|n| n.name)
                    .unwrap_or_else(|| "?".to_string());
                let target_name = session
                    .stores
                    .graph_store
                    .get_node(&edge.target_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|n| n.name)
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "  {} --[{:?}]--> {} (conf: {:.2})",
                    source_name, edge.edge_type, target_name, edge.confidence
                );
                total += 1;
            }
        }
    }

    if total == 0 {
        println!("  No edges in the store.");
    } else {
        println!("\n  {} edge(s) total.", total);
    }
}

async fn cmd_history(session: &Session, args: &str) -> OnyxResult<()> {
    let name = args.trim();
    let node = match find_node_by_name(&session.stores, name).await {
        Some(n) => n,
        None => {
            println!("  Node '{}' not found.", name);
            return Ok(());
        }
    };

    let versions = session.stores.history_store.list_versions(&node.id).await?;

    println!(
        "  Version history for '{}' ({} versions):\n",
        node.name,
        versions.len()
    );

    if versions.is_empty() {
        println!("  (no versions recorded)");
    } else {
        for v in &versions {
            println!(
                "  {} | {} | {} | {} | {} lines",
                &v.version_id[..v.version_id.len().min(16)],
                v.timestamp.format("%Y-%m-%d %H:%M:%S"),
                v.author.as_deref().unwrap_or("system"),
                v.message.as_deref().unwrap_or("(no message)"),
                v.diff.lines_changed()
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn find_node_by_name(
    stores: &TransactionManager,
    name: &str,
) -> Option<onyx::model::node::Node> {
    let all = stores.graph_store.all_nodes().await;
    // Exact match first
    for node in &all {
        if node.name == name {
            return stores.graph_store.get_node(&node.id).await.ok().flatten();
        }
    }
    // Case-insensitive partial match
    let name_lower = name.to_lowercase();
    for node in &all {
        if node.name.to_lowercase().contains(&name_lower) {
            return stores.graph_store.get_node(&node.id).await.ok().flatten();
        }
    }
    None
}

fn parse_edge_types(input: &str) -> Vec<EdgeType> {
    input
        .split(',')
        .filter_map(|s| match s.trim().to_lowercase().as_str() {
            "calls" | "call" => Some(EdgeType::Calls),
            "imports" | "import" => Some(EdgeType::Imports),
            "defines" | "define" => Some(EdgeType::Defines),
            "contains" | "contain" => Some(EdgeType::Contains),
            "tests" | "test" | "testsof" => Some(EdgeType::TestsOf),
            "documents" | "docs" | "doc" => Some(EdgeType::Documents),
            "depends" | "dependson" => Some(EdgeType::DependsOn),
            "implements" | "impl" => Some(EdgeType::Implements),
            "configures" | "config" => Some(EdgeType::Configures),
            _ => {
                eprintln!("  Unknown edge type: '{}'", s.trim());
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Standalone ingest (non-interactive)
// ---------------------------------------------------------------------------

async fn run_ingest(path: &PathBuf) -> OnyxResult<()> {
    let source = std::fs::read_to_string(path)?;
    let units = parse_rust_source(&source, &path.to_string_lossy());

    println!("Parsed {} code entities:", units.len());
    for unit in &units {
        println!("  - {} ({:?})", unit.name, unit.kind);
    }

    let embedder = BagOfWordsEmbedder::from_corpus(
        &units.iter().map(|u| u.content.as_str()).collect::<Vec<_>>(),
        100,
    );

    let mut stores = TransactionManager::new();
    let results = ingest_codebase(&mut stores, &units, &embedder).await?;

    println!("\nIngested {} nodes", results.len());
    let stats = stores.stats();
    println!("Store stats: {}", stats);

    Ok(())
}

// ---------------------------------------------------------------------------
// Demo (non-interactive walkthrough)
// ---------------------------------------------------------------------------

/// Run a full end-to-end demo with a synthetic codebase.
async fn run_demo() -> OnyxResult<()> {
    println!("=== Onyx Demo: Graph-Native Vector Memory ===\n");

    // Build synthetic codebase
    let units = build_synthetic_codebase();

    // Create embedder from the codebase
    let corpus: Vec<&str> = units.iter().map(|u| u.content.as_str()).collect();
    let embedder = BagOfWordsEmbedder::from_corpus(&corpus, 100);

    // Ingest everything
    let mut stores = TransactionManager::new();
    println!("Phase 1: Ingesting {} code artifacts...", units.len());
    let results = ingest_codebase(&mut stores, &units, &embedder).await?;

    for result in &results {
        let node = stores.graph_store.get_node(&result.node_id).await?.unwrap();
        println!("  Ingested: {} ({})", node.name, result.version_id);
    }

    let stats = stores.stats();
    println!("\nStore stats: {}\n", stats);

    // --- Demo 1: Function-level traceability ---
    println!("=== Demo 1: Function-Level Traceability ===\n");
    println!("Query: 'What functions are related to calculate_total?'\n");

    // Find calculate_total by name
    let calc_node = stores
        .graph_store
        .nodes_by_type(&NodeType::CodeEntity(
            onyx::model::node::CodeEntityKind::Function,
        ))
        .await
        .into_iter()
        .find(|n| n.name == "calculate_total");

    if let Some(node) = calc_node {
        println!("Found: {} (ID: {})", node.name, node.id);

        // Traverse from calculate_total
        let traversal = stores
            .graph_store
            .traverse(&node.id, Some(&[EdgeType::Calls]), 3)
            .await?;

        println!("Call graph traversal (depth 3):");
        for (nid, depth) in &traversal.nodes {
            if let Some(n) = stores.graph_store.get_node(nid).await? {
                println!("  {} {} (depth {})", "  ".repeat(*depth), n.name, depth);
            }
        }

        // Inbound callers
        let callers = stores
            .graph_store
            .get_inbound(&node.id, Some(&[EdgeType::Calls]))
            .await?;
        println!("\nCallers of {}:", node.name);
        for (_, caller) in &callers {
            println!("  <- {}", caller.name);
        }
    }

    // --- Demo 2: Business-logic discovery ---
    println!("\n=== Demo 2: Impact Analysis ===\n");
    println!("Query: 'If apply_discount changes, what is affected?'\n");

    let discount_node = stores
        .graph_store
        .nodes_by_type(&NodeType::CodeEntity(
            onyx::model::node::CodeEntityKind::Function,
        ))
        .await
        .into_iter()
        .find(|n| n.name == "apply_discount");

    if let Some(node) = discount_node {
        let affected = impact_analysis(&stores, &node.id, 3).await?;
        println!("Impact analysis for '{}':", node.name);
        for (_, name, depth) in &affected {
            println!("  {} {} (distance {})", ">>>".repeat(*depth), name, depth);
        }

        // Find covering tests
        let tests = find_covering_tests(&stores, &node.id, 2).await?;
        println!("\nTests covering '{}':", node.name);
        if tests.is_empty() {
            println!("  (no direct tests found)");
        }
        for test in &tests {
            println!("  - {} (score: {:.2})", test.name, test.score);
        }
    }

    // --- Demo 3: Semantic search ---
    println!("\n=== Demo 3: Semantic Search with Graph Expansion ===\n");
    println!("Query: Searching for code related to 'payment processing'...\n");

    let query_embedding = embedder.embed("payment total price calculate");
    let options = QueryOptions {
        top_k: 3,
        max_depth: 1,
        edge_types: Some(vec![EdgeType::Calls, EdgeType::Imports]),
        include_history: true,
        ..Default::default()
    };

    let result = execute_query(&stores, Some(&query_embedding.values), &options).await?;
    println!(
        "Found {} results ({} nodes examined, {}ms):",
        result.items.len(),
        result.nodes_examined,
        result.query_time_ms
    );
    for item in &result.items {
        println!(
            "  [{:.3}] {} (depth {}, source: {:?})",
            item.score, item.name, item.depth, item.source
        );
        for v in &item.versions {
            println!(
                "         v{}: {} ({} lines changed)",
                v.version_id.chars().take(12).collect::<String>(),
                v.message.as_deref().unwrap_or("no message"),
                v.lines_changed
            );
        }
    }

    // --- Demo 4: Temporal versioning & bug fix propagation ---
    println!("\n=== Demo 4: Temporal Versioning (Bug Fix Propagation) ===\n");

    // Simulate a bug fix to apply_discount
    let discount_node2 = stores
        .graph_store
        .nodes_by_type(&NodeType::CodeEntity(
            onyx::model::node::CodeEntityKind::Function,
        ))
        .await
        .into_iter()
        .find(|n| n.name == "apply_discount");

    if let Some(node) = discount_node2 {
        let node_id = node.id;

        // Get the current version
        let current_versions = stores.history_store.list_versions(&node_id).await?;
        let parent_version = current_versions.last().unwrap().version_id.clone();

        println!("Scenario: Developer discovers a rounding bug in apply_discount.");
        println!("Recording bug fix as version 2...\n");

        // Record a bug fix version
        let bugfix_content = "pub fn apply_discount(amount: f64) -> f64 {\n    let rules = get_discount_rules();\n    let discounted = rules.iter().fold(amount, |acc, rule| rule.apply(acc));\n    (discounted * 100.0).round() / 100.0 // Fix: round to 2 decimal places\n}";

        let bugfix_version = onyx::model::version::VersionEntry::content_change(
            node_id,
            parent_version,
            bugfix_content,
            2,
            1,
        )
        .with_message("Fix rounding bug in discount calculation")
        .with_author("developer@example.com")
        .with_commit("fix789");

        stores.execute(onyx::store::transaction::TransactionOp::RecordVersion(
            bugfix_version,
        ))?;

        // Record a second improvement version
        let versions_now = stores.history_store.list_versions(&node_id).await?;
        let parent_v2 = versions_now.last().unwrap().version_id.clone();

        let perf_content = "pub fn apply_discount(amount: f64) -> f64 {\n    let rules = get_discount_rules();\n    let discounted = rules.iter().fold(amount, |acc, rule| rule.apply(acc));\n    (discounted * 100.0).round() / 100.0\n}";

        let perf_version = onyx::model::version::VersionEntry::content_change(
            node_id,
            parent_v2,
            perf_content,
            1,
            1,
        )
        .with_message("Add caching for discount rules lookup")
        .with_author("developer@example.com")
        .with_commit("perf012");

        stores.execute(onyx::store::transaction::TransactionOp::RecordVersion(
            perf_version,
        ))?;

        // Show the full version chain
        let all_versions = stores.history_store.list_versions(&node_id).await?;
        println!(
            "Version history for 'apply_discount' ({} versions):\n",
            all_versions.len()
        );

        for (i, v) in all_versions.iter().enumerate() {
            let marker = if i == all_versions.len() - 1 {
                " (HEAD)"
            } else {
                ""
            };
            println!(
                "  v{} {} | {} | {} | {} | {} lines changed{}",
                i + 1,
                &v.version_id[..v.version_id.len().min(12)],
                v.timestamp.format("%H:%M:%S"),
                v.author.as_deref().unwrap_or("system"),
                v.message.as_deref().unwrap_or("(no message)"),
                v.diff.lines_changed(),
                marker
            );
        }

        // Show impact: the bug fix affects everything upstream
        println!("\nImpact of this change (what depends on apply_discount?):");
        let affected = impact_analysis(&stores, &node_id, 3).await?;
        if affected.is_empty() {
            println!("  No downstream impact.");
        } else {
            for (_, aff_name, dist) in &affected {
                println!("  {} {} (distance {})", ">".repeat(*dist), aff_name, dist);
            }
        }
        println!(
            "\n  Result: Bug fix to apply_discount propagates to {} downstream functions.",
            affected.len()
        );
        println!(
            "  All callers should be re-tested: {:?}",
            affected
                .iter()
                .map(|(_, n, _)| n.as_str())
                .collect::<Vec<_>>()
        );
    }

    println!("\n=== Demo Complete ===");
    println!("\nOnyx demonstrated:");
    println!("  1. Structural traversal (call graphs, imports)");
    println!("  2. Impact analysis (change propagation)");
    println!("  3. Semantic search + graph expansion (multi-hop retrieval)");
    println!("  4. Test coverage mapping");
    println!("  5. Version history tracking");
    println!("  6. Temporal versioning (bug fix propagation)");

    Ok(())
}

/// Build a synthetic codebase for demo purposes.
fn build_synthetic_codebase() -> Vec<CodeUnit> {
    use onyx::model::node::{CodeEntityKind, Language, Visibility};

    vec![
        CodeUnit {
            name: "calculate_total".to_string(),
            content: "pub fn calculate_total(items: &[Item], tax_rate: f64) -> f64 {\n    let subtotal = items.iter().map(|i| i.price * i.quantity as f64).sum::<f64>();\n    let discount = apply_discount(subtotal);\n    discount * (1.0 + tax_rate)\n}".to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/billing/calculator.rs".to_string(),
            line_range: Some((10, 15)),
            signature: Some("pub fn calculate_total(items: &[Item], tax_rate: f64) -> f64".to_string()),
            visibility: Visibility::Public,
            module_path: vec!["billing".to_string(), "calculator".to_string()],
            commit_id: Some("abc123".to_string()),
            branch: Some("main".to_string()),
        },
        CodeUnit {
            name: "apply_discount".to_string(),
            content: "pub fn apply_discount(amount: f64) -> f64 {\n    let rules = get_discount_rules();\n    rules.iter().fold(amount, |acc, rule| rule.apply(acc))\n}".to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/billing/discount.rs".to_string(),
            line_range: Some((5, 8)),
            signature: Some("pub fn apply_discount(amount: f64) -> f64".to_string()),
            visibility: Visibility::Public,
            module_path: vec!["billing".to_string(), "discount".to_string()],
            commit_id: Some("abc123".to_string()),
            branch: Some("main".to_string()),
        },
        CodeUnit {
            name: "get_discount_rules".to_string(),
            content: "fn get_discount_rules() -> Vec<DiscountRule> {\n    vec![\n        DiscountRule::percentage(10.0, 100.0),\n        DiscountRule::fixed(5.0, 50.0),\n    ]\n}".to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/billing/discount.rs".to_string(),
            line_range: Some((12, 18)),
            signature: Some("fn get_discount_rules() -> Vec<DiscountRule>".to_string()),
            visibility: Visibility::Private,
            module_path: vec!["billing".to_string(), "discount".to_string()],
            commit_id: Some("abc123".to_string()),
            branch: Some("main".to_string()),
        },
        CodeUnit {
            name: "process_payment".to_string(),
            content: "pub fn process_payment(order: &Order) -> PaymentResult {\n    let total = calculate_total(&order.items, order.tax_rate);\n    let charge = payment_gateway::charge(order.payment_method, total);\n    record_transaction(order.id, total, charge.status)\n}".to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/payment/processor.rs".to_string(),
            line_range: Some((20, 25)),
            signature: Some("pub fn process_payment(order: &Order) -> PaymentResult".to_string()),
            visibility: Visibility::Public,
            module_path: vec!["payment".to_string(), "processor".to_string()],
            commit_id: Some("def456".to_string()),
            branch: Some("main".to_string()),
        },
        CodeUnit {
            name: "record_transaction".to_string(),
            content: "fn record_transaction(order_id: Uuid, amount: f64, status: ChargeStatus) -> PaymentResult {\n    let tx = Transaction::new(order_id, amount, status);\n    db::insert(&tx)?;\n    PaymentResult::from(tx)\n}".to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/payment/ledger.rs".to_string(),
            line_range: Some((8, 13)),
            signature: Some("fn record_transaction(order_id: Uuid, amount: f64, status: ChargeStatus) -> PaymentResult".to_string()),
            visibility: Visibility::Private,
            module_path: vec!["payment".to_string(), "ledger".to_string()],
            commit_id: Some("def456".to_string()),
            branch: Some("main".to_string()),
        },
        CodeUnit {
            name: "validate_order".to_string(),
            content: "pub fn validate_order(order: &Order) -> Result<(), ValidationError> {\n    if order.items.is_empty() {\n        return Err(ValidationError::EmptyOrder);\n    }\n    for item in &order.items {\n        validate_item(item)?;\n    }\n    Ok(())\n}".to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/validation/order.rs".to_string(),
            line_range: Some((1, 9)),
            signature: Some("pub fn validate_order(order: &Order) -> Result<(), ValidationError>".to_string()),
            visibility: Visibility::Public,
            module_path: vec!["validation".to_string(), "order".to_string()],
            commit_id: Some("ghi789".to_string()),
            branch: Some("main".to_string()),
        },
        CodeUnit {
            name: "validate_item".to_string(),
            content: "fn validate_item(item: &Item) -> Result<(), ValidationError> {\n    if item.price < 0.0 {\n        return Err(ValidationError::NegativePrice);\n    }\n    if item.quantity == 0 {\n        return Err(ValidationError::ZeroQuantity);\n    }\n    Ok(())\n}".to_string(),
            kind: CodeEntityKind::Function,
            language: Language::Rust,
            file_path: "src/validation/item.rs".to_string(),
            line_range: Some((1, 9)),
            signature: Some("fn validate_item(item: &Item) -> Result<(), ValidationError>".to_string()),
            visibility: Visibility::Private,
            module_path: vec!["validation".to_string(), "item".to_string()],
            commit_id: Some("ghi789".to_string()),
            branch: Some("main".to_string()),
        },
    ]
}
