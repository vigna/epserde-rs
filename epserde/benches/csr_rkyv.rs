/*
 * SPDX-FileCopyrightText: 2026 epserde contributors
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Best-effort rkyv 0.8 comparison for the CSR benchmark.
//!
//! rkyv is another zero-copy framework: `access_unchecked` hands back a
//! `&ArchivedRkyvCsr` that references the archived byte buffer without a
//! per-field deserialization pass, analogous to ε-serde's ε-copy mmap. Here we
//! read the archive into an aligned buffer and time access + traversal, to sit
//! beside the ε-serde numbers in the evaluation.
//!
//! This target is included only if it builds against the pinned toolchain; if
//! not, it is dropped and the reason recorded in the evaluation notes.

use std::path::{Path, PathBuf};

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use rkyv::primitive::ArchivedUsize;
use rkyv::rancor::Error;
use rkyv::util::AlignedVec;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize)]
struct RkyvCsr {
    offsets: Vec<usize>,
    succ: Vec<usize>,
}

fn gen_csr(n: usize, m: usize) -> RkyvCsr {
    assert!(n >= 1);
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
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let r = (state >> 33) % n_u64;
            succ.push(usize::try_from(r).expect("neighbor id < n fits in usize"));
        }
        total += deg;
        offsets.push(total);
    }
    debug_assert_eq!(total, m);
    RkyvCsr { offsets, succ }
}

fn serialize_to(path: &Path, g: &RkyvCsr) -> usize {
    let bytes = rkyv::to_bytes::<Error>(g).expect("rkyv to_bytes");
    std::fs::write(path, &bytes).expect("write");
    bytes.len()
}

fn read_aligned(path: &Path) -> AlignedVec {
    let raw = std::fs::read(path).expect("read");
    let mut av = AlignedVec::with_capacity(raw.len());
    av.extend_from_slice(&raw);
    av
}

#[inline]
fn traverse(offsets: &[ArchivedUsize], succ: &[ArchivedUsize]) -> usize {
    let mut acc: usize = 0;
    for w in offsets.windows(2) {
        // rkyv's default 32-bit archived usize: to_native() yields u32; widen to usize.
        let lo = usize::try_from(w[0].to_native()).expect("offset fits usize");
        let hi = usize::try_from(w[1].to_native()).expect("offset fits usize");
        for s in &succ[lo..hi] {
            acc = acc.wrapping_add(usize::try_from(s.to_native()).expect("succ fits usize"));
        }
    }
    acc
}

fn bench_rkyv(c: &mut Criterion) {
    let mut sizes = vec![(1_000usize, 50_000usize), (100_000usize, 5_000_000usize)];
    if std::env::var_os("EPS_BENCH_BIG").is_some() {
        sizes.push((1_000_000usize, 20_000_000usize));
    }
    let dir = std::env::temp_dir();
    let cases: Vec<(usize, usize, PathBuf)> = sizes
        .into_iter()
        .map(|(n, m)| {
            let g = gen_csr(n, m);
            let path = dir.join(format!("rkyv_csr_{n}_{m}.bin"));
            let fb = serialize_to(&path, &g);
            eprintln!("[rkyv] n={n} m={m} archive={fb} bytes -> {path:?}");
            (n, m, path)
        })
        .collect();

    {
        let mut g = c.benchmark_group("rkyv_load");
        g.sample_size(30);
        for (n, m, path) in &cases {
            let id = format!("{n}x{m}");
            g.bench_with_input(BenchmarkId::from_parameter(&id), path, |b, path| {
                b.iter(|| {
                    let av = read_aligned(path);
                    // SAFETY: `av` holds bytes produced by `rkyv::to_bytes` for `RkyvCsr`
                    // and is 16-aligned by `AlignedVec`, satisfying archive access.
                    let archived =
                        unsafe { rkyv::access_unchecked::<ArchivedRkyvCsr>(&av) };
                    black_box(archived);
                })
            });
        }
        g.finish();
    }

    {
        let mut g = c.benchmark_group("rkyv_first_touch");
        g.sample_size(30);
        for (n, m, path) in &cases {
            let id = format!("{n}x{m}");
            g.throughput(Throughput::Elements(u64::try_from(*m).expect("m fits")));
            g.bench_with_input(BenchmarkId::from_parameter(&id), path, |b, path| {
                b.iter_batched(
                    || read_aligned(path),
                    |av| {
                        // SAFETY: as above; `av` outlives this closure body.
                        let archived =
                            unsafe { rkyv::access_unchecked::<ArchivedRkyvCsr>(&av) };
                        black_box(traverse(
                            archived.offsets.as_slice(),
                            archived.succ.as_slice(),
                        ))
                    },
                    BatchSize::PerIteration,
                )
            });
        }
        g.finish();
    }
}

criterion_group!(benches, bench_rkyv);
criterion_main!(benches);
