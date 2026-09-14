//! Depth-first visits on a directed Erdős–Rényi graph stored as a list of
//! adjacency lists (`Vec<Vec<u64>>`), comparing:
//!
//! - the native, fully owned `Vec<Vec<u64>>` (baseline);
//! - the ε-copy deserialized form produced by ε-serde, `Vec<&[u64]>`, whose
//!   entries are ordinary fat pointers into the loaded data;
//! - the zero-copy archived form produced by rkyv,
//!   `ArchivedVec<ArchivedVec<u64>>`, whose entries are relative pointers that
//!   must be resolved (base + offset) on every access.
//!
//! Two visit variants are timed. The *standard* iterative DFS keeps a stack of
//! `(node, position)` pairs and fetches the adjacency list of the top node at
//! every step, as in Tarjan-style algorithms; this resolves one pointer per
//! arc. The *cached* variant resolves the adjacency list once per node and
//! keeps the slice on the stack, so pointer resolution happens once per node.

use std::{
    hint::black_box,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Result;
use clap::Parser;
use epserde_rkyv_bench::{Fixture, Graph, erdos_renyi, random_regular};

#[derive(Parser, Debug)]
#[command(about = "DFS speed: ε-serde ε-copy vs rkyv zero-copy")]
struct Args {
    /// Number of nodes (keep the graph within the cache to expose pointer-resolution costs).
    #[arg(short, long, default_value_t = 10_000)]
    nodes: usize,
    /// Average out-degree (arc probability is degree / nodes); in chase mode,
    /// the exact out-degree (default 20, or 1 in chase mode).
    #[arg(short, long)]
    degree: Option<f64>,
    /// Number of timed repetitions per representation.
    #[arg(short, long, default_value_t = 20)]
    repeats: usize,
    /// Seed for the random graph.
    #[arg(short, long, default_value_t = 0)]
    seed: u64,
    /// Directory where the serialized files are written.
    #[arg(long, default_value = "/tmp")]
    dir: PathBuf,
    /// Memory-map the serialized files instead of reading them into RAM.
    #[arg(long)]
    mmap: bool,
    /// Random pointer chase: build a random regular digraph (a random cyclic
    /// permutation when the degree is 1) and follow successors for this many
    /// steps instead of running depth-first visits.
    #[arg(long, value_name = "STEPS")]
    chase: Option<usize>,
}

/// Standard iterative DFS: the stack holds `(node, next position)` and the
/// adjacency list of the top node is fetched again at every step.
fn dfs_standard<G: Graph + ?Sized>(g: &G) -> (usize, usize) {
    let n = g.num_nodes();
    let mut visited = vec![false; n];
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let (mut nodes, mut arcs) = (0usize, 0usize);
    for root in 0..n {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        nodes += 1;
        stack.push((root, 0));
        while let Some(&mut (node, ref mut pos)) = stack.last_mut() {
            let succ = g.successors(node);
            if *pos < succ.len() {
                let v = succ[*pos] as usize;
                *pos += 1;
                arcs += 1;
                if !visited[v] {
                    visited[v] = true;
                    nodes += 1;
                    stack.push((v, 0));
                }
            } else {
                stack.pop();
            }
        }
    }
    (nodes, arcs)
}

/// DFS caching the adjacency slice on the stack: one pointer resolution per
/// node instead of one per arc.
fn dfs_cached<G: Graph + ?Sized>(g: &G) -> (usize, usize) {
    let n = g.num_nodes();
    let mut visited = vec![false; n];
    let mut stack: Vec<std::slice::Iter<'_, u64>> = Vec::new();
    let (mut nodes, mut arcs) = (0usize, 0usize);
    for root in 0..n {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        nodes += 1;
        stack.push(g.successors(root).iter());
        while let Some(iter) = stack.last_mut() {
            match iter.next() {
                Some(&v) => {
                    arcs += 1;
                    let v = v as usize;
                    if !visited[v] {
                        visited[v] = true;
                        nodes += 1;
                        stack.push(g.successors(v).iter());
                    }
                }
                None => {
                    stack.pop();
                }
            }
        }
    }
    (nodes, arcs)
}

/// Follows `steps` successors starting from node 0, picking the `i % degree`-th
/// successor at step `i` (the index does not depend on the current node, so
/// only the pointer resolution is on the critical path).
fn pointer_chase<G: Graph + ?Sized>(g: &G, steps: usize, degree: usize) -> u64 {
    let mut x = 0usize;
    for i in 0..steps {
        x = g.successors(x)[i % degree] as usize;
    }
    x as u64
}

fn time<G: Graph + ?Sized, R>(name: &str, g: &G, repeats: usize, f: impl Fn(&G) -> R) -> Duration
where
    R: Default,
{
    let mut best = Duration::MAX;
    let mut result = R::default();
    for _ in 0..repeats {
        let start = Instant::now();
        result = f(black_box(g));
        best = best.min(start.elapsed());
    }
    black_box(result);
    println!("  {name:<14} {best:>12.3?}");
    best
}

fn main() -> Result<()> {
    let args = Args::parse();
    let Args {
        nodes,
        degree,
        repeats,
        seed,
        dir,
        mmap,
        chase,
    } = args;

    let start = Instant::now();
    let graph = if chase.is_some() {
        let degree = degree.unwrap_or(1.0) as usize;
        eprintln!(
            "Generating random {degree}-regular digraph on {nodes} nodes with seed {seed}..."
        );
        random_regular(nodes, degree, seed)
    } else {
        let degree = degree.unwrap_or(20.0);
        eprintln!("Generating G({nodes}, {degree}/{nodes}) with seed {seed}...");
        erdos_renyi(nodes, degree, seed)
    };
    let arcs: usize = graph.iter().map(Vec::len).sum();
    eprintln!(
        "Generated {nodes} nodes, {arcs} arcs (avg out-degree {:.2}) in {:.2?}",
        arcs as f64 / nodes as f64,
        start.elapsed()
    );

    let start = Instant::now();
    let fixture = Fixture::new(graph, &dir, mmap)?;
    let (eps_size, rkyv_size) = fixture.file_sizes(&dir)?;
    eprintln!(
        "Serialized ({eps_size} bytes ε-serde, {rkyv_size} bytes rkyv), loaded and checked in {:.2?}",
        start.elapsed()
    );
    let graph = &fixture.graph;
    let eps_ref = fixture.eps();
    let rkyv_graph = fixture.rkyv();

    if mmap {
        // Touch every page once so page faults are not counted.
        let _ = black_box(dfs_cached(eps_ref));
        let _ = black_box(dfs_cached(rkyv_graph));
    }

    println!(
        "\n{nodes} nodes, {arcs} arcs, {} ({repeats} repetitions, best time)",
        if mmap { "memory-mapped" } else { "in memory" }
    );

    if let Some(steps) = chase {
        let degree = degree.unwrap_or(1.0) as usize;
        println!("\nRandom pointer chase, {steps} steps, degree {degree}:");
        let t_native = time("native", graph, repeats, |g| {
            pointer_chase(g, steps, degree)
        });
        let t_eps = time("ε-serde", eps_ref, repeats, |g| {
            pointer_chase(g, steps, degree)
        });
        let t_rkyv = time("rkyv", rkyv_graph, repeats, |g| {
            pointer_chase(g, steps, degree)
        });
        println!(
            "  per step: native {:.3} ns, ε-serde {:.3} ns, rkyv {:.3} ns",
            t_native.as_nanos() as f64 / steps as f64,
            t_eps.as_nanos() as f64 / steps as f64,
            t_rkyv.as_nanos() as f64 / steps as f64
        );
        println!(
            "  rkyv / ε-serde = {:.3}   ε-serde / native = {:.3}",
            t_rkyv.as_secs_f64() / t_eps.as_secs_f64(),
            t_eps.as_secs_f64() / t_native.as_secs_f64()
        );
        return Ok(());
    }

    println!("\nStandard DFS (stack of (node, position); one resolution per arc):");
    let t_native = time("native", graph, repeats, dfs_standard);
    let t_eps = time("ε-serde", eps_ref, repeats, dfs_standard);
    let t_rkyv = time("rkyv", rkyv_graph, repeats, dfs_standard);
    println!(
        "  rkyv / ε-serde = {:.3}   ε-serde / native = {:.3}",
        t_rkyv.as_secs_f64() / t_eps.as_secs_f64(),
        t_eps.as_secs_f64() / t_native.as_secs_f64()
    );

    println!("\nCached DFS (stack of slice iterators; one resolution per node):");
    let t_native = time("native", graph, repeats, dfs_cached);
    let t_eps = time("ε-serde", eps_ref, repeats, dfs_cached);
    let t_rkyv = time("rkyv", rkyv_graph, repeats, dfs_cached);
    println!(
        "  rkyv / ε-serde = {:.3}   ε-serde / native = {:.3}",
        t_rkyv.as_secs_f64() / t_eps.as_secs_f64(),
        t_eps.as_secs_f64() / t_native.as_secs_f64()
    );

    Ok(())
}
