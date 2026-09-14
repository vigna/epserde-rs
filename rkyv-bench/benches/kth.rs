//! Two access patterns, one separate benchmark per representation (native
//! `Vec<Vec<u64>>`, ε-serde's ε-copy `Vec<&[u64]>`, rkyv's
//! `ArchivedVec<ArchivedVec<u64>>`), six benchmarks in total:
//!
//! - `kth_successor`: retrieval of the k-th successor of random nodes of an
//!   Erdős–Rényi graph (independent queries, throughput-bound);
//! - `pointer_chase`: a chase `x = successors(x)[i % d]` over a random
//!   `d`-regular digraph built from random cyclic permutations (dependent
//!   accesses, latency-bound).
//!
//! Every access goes through the successor-list pointer (no caching of
//! resolved slices), and indices are passed through `black_box` so the
//! compiler cannot hoist or batch resolutions.
//!
//! Environment: `KTH_SIZES` (comma-separated node counts for both groups;
//! by default `kth_successor` runs at 1 000 000 nodes, where the working set
//! is out of cache, and `pointer_chase` at 1 000 nodes, where it is in L1),
//! `KTH_DIR` (where the serialized files go, default the system temp dir),
//! `KTH_MMAP` (memory-map the files instead of reading them into RAM).

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use epserde_rkyv_bench::{Fixture, Graph, erdos_renyi, random_regular};
use rand::{Rng, SeedableRng, rngs::SmallRng};

const DEGREE: usize = 20;
const QUERIES: usize = 1 << 16;
const STEPS: usize = 1 << 20;
const DEFAULT_KTH_SIZES: &[usize] = &[1_000_000];
const DEFAULT_CHASE_SIZES: &[usize] = &[1_000];

fn config(default_sizes: &[usize]) -> (Vec<usize>, std::path::PathBuf, bool) {
    let sizes = match std::env::var("KTH_SIZES") {
        Ok(s) => s
            .split(',')
            .map(|x| x.trim().parse().expect("size"))
            .collect(),
        Err(_) => default_sizes.to_vec(),
    };
    let dir = std::env::var_os("KTH_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    (sizes, dir, std::env::var_os("KTH_MMAP").is_some())
}

fn queries(graph: &[Vec<u64>], seed: u64) -> Vec<(u32, u32)> {
    let mut rng = SmallRng::seed_from_u64(seed);
    let n = graph.len();
    let mut out = Vec::with_capacity(QUERIES);
    while out.len() < QUERIES {
        let u = rng.random_range(0..n);
        let d = graph[u].len();
        if d > 0 {
            out.push((u as u32, rng.random_range(0..d) as u32));
        }
    }
    out
}

/// Sums the k-th successor over all queries; each query resolves the
/// successor-list pointer of its node.
#[inline(never)]
fn kth<G: Graph + ?Sized>(g: &G, queries: &[(u32, u32)]) -> u64 {
    let mut sum = 0u64;
    for &(u, k) in queries {
        let (u, k) = black_box((u as usize, k as usize));
        sum = sum.wrapping_add(g.successors(u)[k]);
    }
    sum
}

/// Follows `STEPS` successors from node 0, taking the `i % DEGREE`-th at
/// step `i`; each step depends on the previous one.
#[inline(never)]
fn chase<G: Graph + ?Sized>(g: &G) -> u64 {
    let mut x = 0usize;
    for i in 0..STEPS {
        x = g.successors(black_box(x))[i % DEGREE] as usize;
    }
    x as u64
}

fn bench_kth(c: &mut Criterion) {
    let (sizes, dir, mmap) = config(DEFAULT_KTH_SIZES);
    let mut group = c.benchmark_group("kth_successor");
    group.throughput(Throughput::Elements(QUERIES as u64));
    for &n in &sizes {
        let graph = erdos_renyi(n, DEGREE as f64, 0);
        let q = queries(&graph, 1);
        let fixture = Fixture::new(graph, &dir, mmap).expect("fixture");
        let native = &fixture.graph;
        let eps = fixture.eps();
        let rkyv = fixture.rkyv();
        let arcs: usize = native.iter().map(Vec::len).sum();
        println!(
            "{n} nodes, {arcs} arcs ({}); rkyv header array at {:p} ({} mod 16)",
            if arcs.is_multiple_of(2) { "even" } else { "odd" },
            rkyv.as_ptr(),
            rkyv.as_ptr() as usize % 16
        );
        let expected = kth(native, &q);
        assert_eq!(kth(eps, &q), expected);
        assert_eq!(kth(rkyv, &q), expected);

        group.bench_with_input(BenchmarkId::new("native", n), &q, |b, q| {
            b.iter(|| kth(black_box(native), q))
        });
        group.bench_with_input(BenchmarkId::new("epserde", n), &q, |b, q| {
            b.iter(|| kth(black_box(eps), q))
        });
        group.bench_with_input(BenchmarkId::new("rkyv", n), &q, |b, q| {
            b.iter(|| kth(black_box(rkyv), q))
        });
    }
    group.finish();
}

fn bench_chase(c: &mut Criterion) {
    let (sizes, dir, mmap) = config(DEFAULT_CHASE_SIZES);
    let mut group = c.benchmark_group("pointer_chase");
    group.throughput(Throughput::Elements(STEPS as u64));
    group.sample_size(20);
    for &n in &sizes {
        let graph = random_regular(n, DEGREE, 0);
        let fixture = Fixture::new(graph, &dir, mmap).expect("fixture");
        let native = &fixture.graph;
        let eps = fixture.eps();
        let rkyv = fixture.rkyv();
        let expected = chase(native);
        assert_eq!(chase(eps), expected);
        assert_eq!(chase(rkyv), expected);

        group.bench_function(BenchmarkId::new("native", n), |b| {
            b.iter(|| chase(black_box(native)))
        });
        group.bench_function(BenchmarkId::new("epserde", n), |b| {
            b.iter(|| chase(black_box(eps)))
        });
        group.bench_function(BenchmarkId::new("rkyv", n), |b| {
            b.iter(|| chase(black_box(rkyv)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_kth, bench_chase);
criterion_main!(benches);
