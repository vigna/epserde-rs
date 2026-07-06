/*
 * SPDX-FileCopyrightText: 2026 epserde contributors
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Benchmark for the CSR (compressed-sparse-row) graph example.
//!
//! Compares full-copy deserialization ([`Deserialize::load_full`], which reads
//! the whole file and materializes an owned `CsrGraph<Vec<usize>>`) against
//! ε-copy memory mapping ([`Deserialize::mmap`] + `uncase`, which maps the file
//! and hands back a `&CsrGraph<&[usize]>` referencing the mapped bytes with no
//! data copy). Peak-RSS is measured out of band by `examples/csr_mem.rs`.
//!
//! Toolchain (for reproducibility): build with stable rustc and
//! `RUSTFLAGS="-C target-cpu=native"` (see nixconfig `pkgs/rustenv.nix`).
//!
//!   cargo bench --bench csr                 # default sizes (small, mid)
//!   EPS_BENCH_BIG=1 cargo bench --bench csr # adds the largest size
//!
//! rkyv comparison: a parallel harness lives in `benches/csr_rkyv.rs` (enabled
//! only if it builds against the pinned toolchain); if that build fails the
//! target is dropped and the reason recorded in the evaluation notes.

use std::path::{Path, PathBuf};

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use epserde::prelude::*;

/// Compressed-sparse-row graph: `(offsets, successors)`. Mirrors the paper's
/// running example. `A = Vec<usize>` when built/owned; ε-copy deserializes to
/// `A = &[usize]`.
#[derive(Epserde)]
struct CsrGraph<A>(A, A);

/// Deterministic CSR generator: `n` nodes, `m` arcs, fixed-seed LCG so runs are
/// reproducible without an RNG dependency. Degrees are spread as evenly as
/// possible; successors are pseudo-random node ids in `[0, n)`.
fn gen_csr(n: usize, m: usize) -> CsrGraph<Vec<usize>> {
    assert!(n >= 1, "need at least one node");
    let n_u64 = u64::try_from(n).expect("n fits in u64");
    let mut offsets = Vec::with_capacity(n + 1);
    let mut succ = Vec::with_capacity(m);
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let base = m / n;
    let extra = m % n;
    offsets.push(0usize);
    let mut total = 0usize;
    for u in 0..n {
        let deg = base + usize::from(u < extra);
        for _ in 0..deg {
            // LCG step (Knuth's MMIX constants); wrapping is the intended RNG arithmetic.
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // (state >> 33) < 2^31, so the reduction and conversion are lossless.
            let r = (state >> 33) % n_u64;
            succ.push(usize::try_from(r).expect("neighbor id < n fits in usize"));
        }
        total += deg;
        offsets.push(total);
    }
    debug_assert_eq!(total, m);
    CsrGraph(offsets, succ)
}

/// Serialize a graph to `path`; returns the number of bytes written.
fn serialize_to(path: &Path, g: &CsrGraph<Vec<usize>>) -> usize {
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    // SAFETY: cursor is a fresh in-memory buffer; nothing aliases it.
    let _ = unsafe { g.serialize(&mut cursor) }.expect("serialize");
    let bytes = cursor.as_bytes();
    std::fs::write(path, bytes).expect("write serialized graph");
    bytes.len()
}

/// Sum every successor exactly once. Accumulates in `usize` (no per-element
/// cast) with wrapping arithmetic; the result is only fed to `black_box`.
#[inline]
fn traverse(offsets: &[usize], succ: &[usize]) -> usize {
    let mut acc: usize = 0;
    for w in offsets.windows(2) {
        for &s in &succ[w[0]..w[1]] {
            acc = acc.wrapping_add(s);
        }
    }
    acc
}

struct Case {
    n: usize,
    m: usize,
    path: PathBuf,
}

fn make_cases() -> Vec<Case> {
    let mut sizes = vec![(1_000usize, 50_000usize), (100_000usize, 5_000_000usize)];
    if std::env::var_os("EPS_BENCH_BIG").is_some() {
        sizes.push((1_000_000usize, 20_000_000usize));
    }
    let dir = std::env::temp_dir();
    sizes
        .into_iter()
        .map(|(n, m)| {
            let g = gen_csr(n, m);
            let path = dir.join(format!("epserde_csr_{n}_{m}.bin"));
            let file_bytes = serialize_to(&path, &g);
            // Precision loss acceptable: MiB figure is for human-readable logging only.
            let mib = file_bytes as f64 / (1024.0 * 1024.0);
            eprintln!("[csr] n={n} m={m} serialized={file_bytes} bytes ({mib:.2} MiB) -> {path:?}");
            Case {
                n,
                m,
                path,
            }
        })
        .collect()
}

fn bench_csr(c: &mut Criterion) {
    let cases = make_cases();

    // --- Load latency: full-copy vs ε-copy mmap ---
    {
        let mut g = c.benchmark_group("load");
        g.sample_size(30); // file I/O per iteration; keep runtime bounded
        for case in &cases {
            let id = format!("{}x{}", case.n, case.m);
            g.bench_with_input(BenchmarkId::new("full_copy", &id), &case.path, |b, path| {
                b.iter(|| {
                    // SAFETY: file was written by this harness in a compatible layout.
                    let graph =
                        unsafe { <CsrGraph<Vec<usize>>>::load_full(path) }.expect("load_full");
                    black_box(&graph);
                })
            });
            g.bench_with_input(BenchmarkId::new("mmap_eps", &id), &case.path, |b, path| {
                b.iter(|| {
                    // SAFETY: as above; file outlives the mapping within the closure.
                    let mc = unsafe { <CsrGraph<Vec<usize>>>::mmap(path, Flags::empty()) }
                        .expect("mmap");
                    black_box(&mc);
                })
            });
        }
        g.finish();
    }

    // --- First-touch traversal: fresh mmap each iteration (pages faulted lazily) ---
    {
        let mut g = c.benchmark_group("first_touch_traverse_mmap");
        g.sample_size(30);
        for case in &cases {
            let id = format!("{}x{}", case.n, case.m);
            g.throughput(Throughput::Elements(u64::try_from(case.m).expect("m fits in u64")));
            g.bench_with_input(BenchmarkId::from_parameter(&id), &case.path, |b, path| {
                b.iter_batched(
                // SAFETY: `path` was written by `serialize_to` in this process in a
                // layout compatible with `CsrGraph<Vec<usize>>`; the returned `MemCase`
                // owns the mapping for the whole iteration, so its bytes stay valid.
                    || unsafe { <CsrGraph<Vec<usize>>>::mmap(path, Flags::empty()) }.expect("mmap"),
                    |mc| {
                        let graph = mc.uncase();
                        black_box(traverse(graph.0, graph.1))
                    },
                    BatchSize::PerIteration,
                )
            });
        }
        g.finish();
    }

    // --- Steady-state traversal: resident data, mmap vs owned full-copy ---
    {
        let mut g = c.benchmark_group("steady_traverse");
        for case in &cases {
            let id = format!("{}x{}", case.n, case.m);
            g.throughput(Throughput::Elements(u64::try_from(case.m).expect("m fits in u64")));

            // SAFETY: `case.path` was written by this harness in a compatible layout;
            // `mc` keeps the mapping alive for as long as `mgraph` borrows from it.
            let mc =
                unsafe { <CsrGraph<Vec<usize>>>::mmap(&case.path, Flags::empty()) }.expect("mmap");
            let mgraph = mc.uncase();
            black_box(traverse(mgraph.0, mgraph.1)); // warm faults
            g.bench_with_input(BenchmarkId::new("mmap_eps", &id), &(), |b, _| {
                b.iter(|| black_box(traverse(mgraph.0, mgraph.1)))
            });

            // SAFETY: `case.path` was written by this harness in a compatible layout;
            // `load_full` fully materializes an owned value, borrowing nothing after return.
            let owned = unsafe { <CsrGraph<Vec<usize>>>::load_full(&case.path) }.expect("load_full");
            black_box(traverse(&owned.0, &owned.1));
            g.bench_with_input(BenchmarkId::new("full_copy", &id), &(), |b, _| {
                b.iter(|| black_box(traverse(&owned.0, &owned.1)))
            });
        }
        g.finish();
    }
}

criterion_group!(benches, bench_csr);
criterion_main!(benches);
