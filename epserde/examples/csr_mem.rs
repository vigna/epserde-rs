/*
 * SPDX-FileCopyrightText: 2026 epserde contributors
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

//! Out-of-band peak-RSS probe for the CSR benchmark (see `benches/csr.rs`).
//!
//! Graph generation and RSS measurement run as SEPARATE processes so a loader's
//! peak resident set (`/proc/self/status` `VmHWM`) excludes the generator's
//! buffers. Usage:
//!
//!   csr_mem prepare <path> <n> <m>   # build + serialize a graph, then exit
//!   csr_mem full    <path>           # load_full + traverse; print VmHWM
//!   csr_mem mmap    <path>           # mmap + uncase + traverse; print VmHWM
//!
//! `VmHWM` is reported twice per loader: right after load (materialization
//! cost) and after a full traversal (which faults in every touched page).

use epserde::prelude::*;

#[derive(Epserde)]
struct CsrGraph<A>(A, A);

/// Deterministic CSR generator (identical to `benches/csr.rs`).
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
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // (state >> 33) < 2^31, so reduction and conversion are lossless.
            let r = (state >> 33) % n_u64;
            succ.push(usize::try_from(r).expect("neighbor id < n fits in usize"));
        }
        total += deg;
        offsets.push(total);
    }
    debug_assert_eq!(total, m);
    CsrGraph(offsets, succ)
}

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

/// Peak resident set size in kB from `/proc/self/status` (`VmHWM`), or 0 if
/// unavailable (non-Linux).
fn vmhwm_kb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            if let Some(tok) = rest.split_whitespace().next() {
                return tok.parse().unwrap_or(0);
            }
        }
    }
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");
    match mode {
        "prepare" => {
            let path = &args[2];
            let n: usize = args[3].parse().expect("n");
            let m: usize = args[4].parse().expect("m");
            let g = gen_csr(n, m);
            let mut cursor = <AlignedCursor<Aligned16>>::new();
            // SAFETY: `cursor` is a fresh in-memory buffer unaliased by anything else.
            let _ = unsafe { g.serialize(&mut cursor) }.expect("serialize");
            let bytes = cursor.as_bytes();
            std::fs::write(path, bytes).expect("write");
            eprintln!("prepared {path}: {} bytes", bytes.len());
        }
        "full" => {
            let path = &args[2];
            // SAFETY: `path` was written by a prior `prepare` run in a compatible layout.
            let g = unsafe { <CsrGraph<Vec<usize>>>::load_full(path) }.expect("load_full");
            let after_load = vmhwm_kb();
            let acc = traverse(&g.0, &g.1);
            let after_traverse = vmhwm_kb();
            println!(
                "mode=full acc={acc} vmhwm_after_load_kB={after_load} vmhwm_after_traverse_kB={after_traverse}"
            );
        }
        "mmap" => {
            let path = &args[2];
            // SAFETY: `path` was written by a prior `prepare` run in a compatible layout;
            // `mc` owns the mapping for the rest of `main`, so `g`'s borrows stay valid.
            let mc = unsafe { <CsrGraph<Vec<usize>>>::mmap(path, Flags::empty()) }.expect("mmap");
            let g = mc.uncase();
            let after_load = vmhwm_kb();
            let acc = traverse(g.0, g.1);
            let after_traverse = vmhwm_kb();
            println!(
                "mode=mmap acc={acc} vmhwm_after_load_kB={after_load} vmhwm_after_traverse_kB={after_traverse}"
            );
        }
        _ => {
            eprintln!("usage: csr_mem prepare <path> <n> <m> | full <path> | mmap <path>");
            std::process::exit(2);
        }
    }
}
