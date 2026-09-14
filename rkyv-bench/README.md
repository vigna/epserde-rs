# ε-serde vs rkyv: depth-first visits on an Erdős–Rényi graph

Standalone benchmark (not a workspace member) measuring the runtime cost of
rkyv's relative-pointer resolution against ε-serde's ε-copy deserialization.

The graph is a directed G(n, p) with p = degree / n, stored as a list of
adjacency lists, `Vec<Vec<u64>>`. Three representations are visited:

- **native**: the owned `Vec<Vec<u64>>` (baseline);
- **ε-serde**: the ε-copy form `Vec<&[u64]>` returned by `load_mem`/`mmap`;
  each entry is an ordinary fat pointer into the loaded bytes;
- **rkyv**: the archived form `ArchivedVec<ArchivedVec<u64>>` obtained with
  `access_unchecked`, built with the `pointer_width_64` feature; each entry is
  a relative pointer (64-bit offset plus 64-bit length) that must be resolved
  against its own address on every access.

Two visits are timed:

- **standard DFS**: stack of `(node, position)`; the adjacency list of the
  node on top of the stack is fetched at every step, so a pointer is resolved
  once per arc (this is the usual structure of Tarjan-style algorithms);
- **cached DFS**: stack of slice iterators; a pointer is resolved once per
  node.

A third workload, the **random pointer chase** (`--chase STEPS`), replaces
the Erdős–Rényi graph with a random `d`-regular digraph whose successor lists
are the images of the node under `d` random cyclic permutations, and follows
`x = successors(x)[i % d]` for STEPS steps. Each step's critical path is then
just header load, pointer resolution, successor load, with nothing else to
hide the resolution behind.

A Criterion benchmark (`cargo bench`, or `./reproduce.sh CORE`) produces
six items: the retrieval of the k-th successor of random nodes of the
Erdős–Rényi graph (`kth_successor`) and a pointer chase over a
permutation-based 20-regular digraph (`pointer_chase`), each for the native,
ε-serde, and rkyv representations. By default the lookups run at 1 M nodes,
where the working set is out of cache and rkyv's header-array alignment
matters, and the chase at 1 000 nodes, where the structure sits in L1 and
the relative-pointer add is visible; `KTH_SIZES` overrides both.
No resolved slice is cached and indices go through `black_box`. The layout
controls used in the investigation below (`RelGraph`, `JoinedGraph`) remain
in the library but are not part of the benchmark.

## Usage

```bash
cargo run --release -- [-n NODES] [-d DEGREE] [-r REPEATS] [-s SEED] [--dir DIR] [--mmap] [--chase STEPS]
taskset -c 2 cargo bench --bench kth            # k-th successor, all sizes
KTH_MMAP=1 taskset -c 2 cargo bench --bench kth # same, memory-mapped files
KTH_DIR=target/data KTH_SIZES=3000000,10000000 taskset -c 2 cargo bench --bench kth
```

Environment switches for the Criterion bench: `KTH_SIZES` (comma-separated
node counts for both groups; defaults 1 000 000 for the lookups and 1 000
for the chase), `KTH_DIR` (where the serialized files go),
`KTH_MMAP` (memory-map instead of reading).

Defaults: 10 000 nodes, average out-degree 20, 20 repetitions (best time is
reported), files written to `/tmp`. Pin to a performance core on hybrid CPUs
(e.g., `taskset -c 2`).

## Results (i7-12700KF, pinned with `taskset -c 2`, in memory, best of N)

Standard DFS times, ε-serde vs rkyv (64-bit offsets):

| nodes     | degree | ε-serde   | rkyv      |
|----------:|-------:|----------:|----------:|
| 1 000     | 5      | 16.5 µs   | 16.3 µs   |
| 1 000     | 20     | 55.2 µs   | 51.2 µs   |
| 1 000     | 50     | 129.0 µs  | 129.5 µs  |
| 1 000     | 200    | 454.5 µs  | 454.0 µs  |
| 10 000    | 5      | 360.9 µs  | 360.9 µs  |
| 10 000    | 20     | 729.0 µs  | 734.0 µs  |
| 10 000    | 50     | 1.408 ms  | 1.417 ms  |
| 10 000    | 200    | 4.804 ms  | 4.806 ms  |
| 100 000   | 20     | 10.75 ms  | 10.78 ms  |
| 100 000   | 200    | 61.72 ms  | 61.60 ms  |
| 1 000 000 | 5      | 168.5 ms  | 160.7 ms  |
| 1 000 000 | 20     | 273.3 ms  | 269.7 ms  |

At the default configuration (10 000 nodes, degree 20) the cached DFS gives
native 648 µs, ε-serde 629 µs, rkyv 620 µs. The native `Vec<Vec<u64>>` is
5–7% slower than both in the standard DFS.

## Interpretation

With 64-bit offsets the two hot loops compile to the same instruction
sequence except for a single instruction: ε-serde loads the data pointer
(`mov (%rax),%rax`), rkyv adds the offset to the header address
(`add (%rax),%rax`). That is one cycle of extra latency per resolution, which
is not measurable against the ~17-cycle dependency chain of a DFS step (stack
top, header, successor, visited bit). The result is a tie within noise, both
in and out of cache; rkyv is marginally ahead on the largest graphs, where
memory traffic dominates and the fat-pointer vector of the ε-copy form is a
separate allocation from the data.

With rkyv's default 32-bit offsets (`pointer_width_32`) the picture is
different: the offset must be sign-extended before the add, the header is 8
bytes instead of 16, and ε-serde is 6–16% faster in cache while rkyv is up to
12% faster out of cache. Those results are not representative of a 64-bit
deployment and are not reported here.

## Random pointer chase (`taskset -c 2`, 10 M steps, best of 5)

Nanoseconds per step:

| nodes     | degree | native | ε-serde | rkyv  | rkyv / ε-serde |
|----------:|-------:|-------:|--------:|------:|---------------:|
| 1 000     | 1      | 4.47   | 3.09    | 3.35  | 1.085          |
| 1 000     | 8      | 5.71   | 4.70    | 4.84  | 1.029          |
| 10 000    | 1      | 9.34   | 7.72    | 7.70  | 0.998          |
| 10 000    | 8      | 10.43  | 9.30    | 9.44  | 1.015          |
| 100 000   | 1      | 31.69  | 27.31   | 22.91 | 0.839          |
| 100 000   | 8      | 34.10  | 31.08   | 30.71 | 0.988          |
| 1 000 000 | 1      | 146.9  | 94.8    | 60.1  | 0.634          |
| 1 000 000 | 8      | 154.8  | 136.6   | 134.7 | 0.986          |
| 1 000     | 16     | 6.12   | 5.27    | 5.53  | 1.048          |
| 1 000     | 64     | 7.13   | 6.24    | 6.50  | 1.042          |
| 1 000     | 256    | 11.88  | 11.19   | 11.47 | 1.025          |
| 10 000    | 16     | 14.14  | 12.84   | 12.23 | 0.953          |
| 10 000    | 64     | 20.39  | 19.45   | 19.70 | 1.013          |
| 10 000    | 256    | 30.88  | 29.90   | 30.31 | 1.014          |
| 100 000   | 16     | 36.48  | 33.52   | 33.64 | 1.003          |
| 100 000   | 64     | 86.42  | 84.74   | 84.36 | 0.995          |
| 100 000   | 256    | 116.1  | 112.9   | 113.2 | 1.002          |
| 1 000 000 | 16     | 161.7  | 144.7   | 144.8 | 1.001          |
| 1 000 000 | 64     | 172.3  | 159.4   | 158.4 | 0.994          |

When everything is in L1 (1 000 nodes, degree 1: 32 KB) a step costs about
14.5 cycles for ε-serde and 15.7 for rkyv: the one-cycle `add` of the
relative pointer is now visible, an 8.5% penalty for rkyv. As soon as the
loads hit L2 or beyond, the cycle disappears in the load latency and the two
tie.

Out of cache with degree 1, rkyv is much faster, but for a reason unrelated
to pointer resolution: ε-serde serializes each inner `Vec<u64>` as an 8-byte
length followed by the data, and the ε-copy form adds a 16-byte fat pointer
per list, so a one-element list costs 32 bytes against rkyv's 24 (16-byte
header plus 8 bytes of data). At 1 M nodes that is 32 MB against 24 MB
around a 25 MB L3; at 100 000 nodes rkyv's 800 KB data region fits in L2
while ε-serde's 1.6 MB does not. With degree 8 the length overhead is
amortized and the two representations tie out of cache as well; with degrees
16 to 256 (length overhead between 6% and 0.4% of the footprint) every
out-of-cache configuration is a tie within 1%, and the in-cache ones (1 000
nodes) keep a 2.5–5% edge for ε-serde that shrinks as the lists grow and the
loads move from L1 to L2.

## k-th successor retrieval (Criterion, `taskset -c 2`, 65 536 random queries per iteration)

Erdős–Rényi graph, degree 20. Time per iteration (Criterion mean):

| nodes     | native   | ε-serde  | rkyv     | rkyv / ε-serde |
|----------:|---------:|---------:|---------:|---------------:|
| 1 000     | 104.6 µs | 102.3 µs | 102.1 µs | 1.00           |
| 10 000    | 147.0 µs | 140.8 µs | 141.7 µs | 1.01           |
| 100 000   | 249.0 µs | 228.5 µs | 229.0 µs | 1.00           |
| 1 000 000 | 432.5 µs | 370.6 µs | 419.5 µs | 1.13           |
| 3 000 000 | 574.1 µs | 514.0 µs | 557.6 µs | 1.08           |
| 10 000 000 | 744.0 µs | 668.2 µs | 667.9 µs | 1.00          |

Independent random queries are throughput-bound: the relative-pointer add is
a fused micro-op and costs nothing, so the three sizes that fit in cache tie
within 1%. At 1 M nodes (176 MB working set) rkyv is 13% slower, and the
controls show why. `relative` uses ε-serde's data buffer with a separate
vector of rkyv-style relative offsets, resolved with an add on every access;
`joined_data_first` replicates rkyv's layout exactly in a single `Vec<u64>`
(all list data, then a 16-byte relative header per node);
`joined_headers_first` is the same with the header region placed before the
data.

| representation (1 M nodes) | time     | L2 demand misses | L2 HW prefetches |
|----------------------------|---------:|-----------------:|-----------------:|
| ε-serde                    | 370.6 µs | 3.34 G           | 0.77 G           |
| relative                   | 370.9 µs | 3.34 G           | 0.77 G           |
| joined_headers_first       | 370.5 µs | 3.30 G           | 0.78 G           |
| joined_data_first          | 420.1 µs | 3.79 G           | 4.88 G           |
| rkyv                       | 417.7 µs | 3.82 G           | 5.11 G           |
| rkyv, memory-mapped        | 420.1 µs |                  |                  |

The `rkyv` and `relative` loops are instruction-for-instruction identical
(the root header resolution is hoisted; each query costs one `add (%r8),%r8`
more than ε-serde), yet `relative` runs at ε-serde's speed. The difference is
not resolution but **alignment of the header array**. rkyv aligns the array
of `ArchivedVec<u64>` headers to `align_of::<ArchivedVec<u64>>()`, which is
8, and writes it right after the list data; the graph above has 19 995 373
arcs, an odd number of words, so the 16-byte headers start 8 bytes into a
16-byte slot and one header in four straddles a cache line. That costs an
extra line fetch on a quarter of the queries (the 13% extra L2 demand
misses), and the line-split loads touch consecutive lines, which is exactly
what trains the L2 streamer: hence the five useless hardware prefetches per
query. `joined_data_first` reproduced it for the same reason (its headers
also follow an odd number of data words), and `joined_headers_first` avoided
it because its headers start at the allocation, which is 16-byte aligned.

A separate experiment at 1 M nodes (not kept in this crate) tested this
directly, with separate header allocations placed below and above the data,
a control whose header array is deliberately shifted by 8 bytes, and the
same graph with one arc added to make the count even:

| representation           | odd arc count | even arc count |
|--------------------------|--------------:|---------------:|
| ε-serde                  | 368.9 µs      | 367.9 µs       |
| rkyv                     | 418.7 µs      | 365.2 µs       |
| split, headers below data | 366.4 µs     | 367.4 µs       |
| split, headers above data | 365.5 µs     | 367.6 µs       |
| split, headers at 8 mod 16 | 424.4 µs    | 423.2 µs       |

Address order is irrelevant; a header array at 8 mod 16 is slow whatever
produces it, and rkyv is slow exactly when its header array lands there,
which for this structure happens on half of the inputs. With an even arc
count rkyv is in fact marginally *faster* than ε-serde on this workload.
ε-serde never hits the problem because its header array is the
`Vec<&[u64]>` allocated at deserialization, which the allocator aligns to 16
bytes.

The size of the effect depends on the hardware. The i7-12700KF has 64-byte
cache lines; Apple M-series processors report 128-byte lines
(`sysctl hw.cachelinesize`), where a header array at 8 mod 16 puts one
header in eight, rather than one in four, across a line boundary.

The effect is confined to a band of working-set sizes. At 10 M nodes (1.8 GB)
all five representations, controls included, tie at about 668 µs (the joined
layouts at 659 µs): every access is then a DRAM access with a TLB miss, and
the extra adjacent line, in the same page and usually the same open DRAM row,
is cheap by comparison. The bump peaks around 1 M nodes
(176 MB), is still 8% at 3 M (530 MB), and is gone at 10 M. Files for the
larger sizes must be kept on disk (`KTH_DIR`), since `/tmp` is a RAM-backed
tmpfs on this machine.

The `pointer_chase` group of the Criterion bench (degree 20, 2^20 dependent
steps) gives, at its default 1 000 nodes, native 6.19 ms, ε-serde 5.47 ms,
rkyv 5.76 ms (rkyv 5.3% slower); at 1 M nodes, native 171.8 ms, ε-serde
154.4 ms, rkyv 157.4 ms, a 2% edge at the limit of run-to-run noise. Both are
consistent with the chase sweep above.

Conclusion: on x86-64 with 64-bit offsets, relative-pointer resolution costs
one cycle per access. It is measurable (about 8%) only in a pure
latency-bound pointer chase whose working set sits in L1; in a depth-first
visit, in throughput-bound random lookups, or as soon as the data leaves L1,
it vanishes. The one out-of-cache workload where ε-serde beats rkyv by a
clear margin (13% on random k-th successor lookups over a 176 MB graph) does
so because rkyv's 8-byte-aligned header array straddles cache lines when an
odd number of words precedes it, which happens on half of the inputs, not
because of the resolution itself; the six-item benchmark prints the arc
count and the alignment of rkyv's header array so the case is known.

---

## Figure

The cost of accessing a zero-copy rkyv archive relative to an ε-copy ε-serde
image of the same adjacency lists, for the two groups of the Criterion
benchmark (`kth_successor` and `pointer_chase`), one figure per machine.

Three independent steps. Each reads its input and writes its output; nothing
is shared but the files between them.

### 1. Measure

On the machine to be measured, from this directory:

```sh
taskset -c 2 cargo bench --bench kth -- --save-baseline full --noplot
```

Writes `target/criterion`, baseline `full`. Pin to a performance core and
keep the machine otherwise idle; macOS has no `taskset`, so there drop
`taskset -c 2` and the run is unpinned. The usual switches apply:
`KTH_SIZES` (the same node counts for both groups, so that the figure shows
each size for both), `KTH_DIR` (keep the files of large sizes on disk),
`KTH_MMAP` (memory-map instead of reading).

Save under a name, as above: a plain `cargo bench --bench kth`, or
`./reproduce.sh`, writes baseline `new`, which criterion moves to `base` at
the next run.

For each size of the `kth_successor` group the benchmark prints a line such
as

```text
1000000 nodes, 19995373 arcs (odd); rkyv header array at 0x7f... (8 mod 16)
```

Note it down with the results: the rkyv bar depends on whether the header
array lands at 8 mod 16, and on the cache-line size of the machine (see
above).

### 2. Extract

```sh
./extract_samples.py --baseline full -o intel.json
```

Pulls every raw timing out of the criterion tree of this directory into one
file; name it after the machine. `target/criterion` does not survive
`cargo clean`; the JSON file does, and it is all the drawing step needs.

The JSON file maps `group/arm/param` (e.g. `pointer_chase/rkyv/1000`) to the
`times` and `iters` criterion recorded; dividing elementwise gives nanoseconds
per iteration (65 536 queries for `kth_successor`, 2^20 steps for
`pointer_chase`).

This step is optional: the drawing script also reads a criterion tree
directly (`./plot_rkyv_overhead.py target/criterion --baseline full`), and
draws the same figure from either.

### 3. Draw

```sh
./plot_rkyv_overhead.py intel.json -o intel_overhead
```

Writes `intel_overhead.pdf` and `.png`. Run it as often as you like: it only
reads.

Each bar is the percentage by which the mean time per iteration of rkyv
exceeds that of ε-serde, for one group and one node count; the groups are on
the x axis, the node counts in the legend. If the two groups were run at
different sizes (by default 1 000 000 nodes for the lookups and 1 000 for the
chase) each group shows only its own bars. Whiskers span a 95% bootstrap
confidence interval of the ratio of the means, from 10 000 resamples of the
raw timings with a fixed seed, so redrawing gives the same figure. The
`pointer_chase` group takes only 20 samples, so its interval is wider.

`--help` lists the options:

| option | effect |
|--------|--------|
| `--width`, `--height` | size in inches (default 3.5 × 2.2) |
| `--font-size` | points (default 8) |
| `--font` | a text face other than the paper's |
| `--color` | colour instead of grayscale |
| `--bench` | a subset of the groups (repeatable) |
| `--subject`, `--baseline-arm` | compare a different pair among `native`, `epserde`, `rkyv` |
| `--baseline` | criterion baseline to read when the source is a directory |

Drawing needs Python 3 with matplotlib and numpy, and Linux Libertine or
Libertinus Serif, the paper's text face
(Fedora: `linux-libertine-fonts`, Debian: `fonts-linuxlibertine`). Only the
machine that draws needs it, not the one that measures. If it is missing the
script says so and names the package rather than silently substituting a
face. If matplotlib does not see a newly installed face, clear its cache:
`rm ~/.cache/matplotlib/fontlist-*.json`.

### Several machines

Repeat steps 1 and 2 on each machine (x86-64 or ARM, Linux or macOS), giving
each JSON file a name of its own, and draw one figure per file:

```sh
./plot_rkyv_overhead.py intel.json -o intel_overhead
./plot_rkyv_overhead.py m1.json -o m1_overhead
```

Only the JSON files need to travel: the machine that draws needs neither
Rust nor the criterion tree. Expect the `kth_successor` bar to differ
between machines, for the reasons above.

### Including it in the paper

The figure targets `epserde.tex`
(`\documentclass[acmsmall,review,anonymous]{acmart}`): Linux Libertine at 8 pt,
which is `\footnotesize` in that 10 pt document. Include it at natural size;
scaling changes the type size and breaks the match.

```latex
\begin{figure}
  \centering
  \includegraphics{intel_overhead}
  \caption{Cost of accessing a zero-copy rkyv archive relative to an
    \eserde{} image of the same adjacency lists: retrieval of the $k$-th
    successor of random nodes of an Erd\H{o}s--R\'enyi graph with $10^6$
    nodes and average degree 20, and a pointer chase over a random
    20-regular digraph with $10^3$ nodes. Whiskers span a 95\% bootstrap
    confidence interval.}
  \label{fig:rkyv-access}
\end{figure}
```

Adjust the sizes in the caption if you ran with `KTH_SIZES`, and say that the
images were memory-mapped if you ran with `KTH_MMAP`.

`acmsmall`'s `\textwidth` is 5.478 in, so `--width 5.478` gives a full-width
figure.

`pdffonts` warns "Mismatch between font type and embedded font file". That is
poppler being fussy about matplotlib's OpenType/CFF wrapper; the fonts are
embedded and the figure renders correctly.
