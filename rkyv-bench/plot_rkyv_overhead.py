#!/usr/bin/env python3
"""Paper figure: the cost of accessing a zero-copy rkyv archive relative to an
ε-copy ε-serde image of the same graph, for the two workloads of the `kth`
Criterion benchmark (random k-th successor retrieval and pointer chasing).

Both are images of the identical `Vec<Vec<u64>>` adjacency lists, so the
comparison isolates the cost of the format. The figure has two bars, one per
workload, each at the single node count that workload was measured at.

USAGE.  It draws from a `samples.json` produced by `extract_samples.py`, or
directly from the criterion tree written by `cargo bench --bench kth`:

    ./plot_rkyv_overhead.py intel.json -o intel_overhead
    ./plot_rkyv_overhead.py target/criterion --baseline full -o intel_overhead

Options:

    --width 5.477               full acmsmall \\textwidth (default 2.638, half
                                of it less a 0.2in gap, for two on a line)
    --bar-width 0.15            thinner bars (default 0.25)
    --color                     colour instead of grayscale
    --bench pointer_chase       a single workload
    --baseline-arm native       measure against native instead of ε-serde
    --baseline full             read a saved criterion baseline (default new)
    --font "Some Family"        a text face other than the paper's

Each bar is the percentage by which the mean time of the subject exceeds the
mean time of the baseline arm; whiskers span a 95% bootstrap confidence
interval of that ratio, resampled from the raw criterion timings (the seed is
fixed, so redrawing gives the same figure).

TYPOGRAPHY.  The target document is ``epserde.tex``, which is
``\\documentclass[acmsmall,review,anonymous]{acmart}``: Linux Libertine text,
Inconsolata typewriter, newtxmath math, a 10pt base size and a 5.477in text
width.  This script therefore sets Linux Libertine for both text and math and
defaults to 8pt, which is ``\\footnotesize`` in a 10pt document.  Include the
result at natural size -- ``\\includegraphics{intel_overhead}`` with no
scaling -- or the type will no longer be 8pt.

Glyphs are embedded as TrueType (``pdf.fonttype = 42``) rather than Type 3,
which some publishers reject.

Linux Libertine (or Libertinus Serif) must be available, either installed
system-wide (Fedora: ``linux-libertine-fonts``; Debian:
``fonts-linuxlibertine``) or as part of TeX Live, whose OpenType files the
script finds with ``kpsewhich`` when the face is not installed.  If matplotlib
does not see a newly installed face, clear the font cache:
``rm ~/.cache/matplotlib/fontlist-*.json``.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

import matplotlib

matplotlib.use("pdf")
import matplotlib.font_manager as fm  # noqa: E402
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

BENCHES = ["kth_successor", "pointer_chase"]
ARMS = ["native", "epserde", "rkyv"]

#: How each workload is named on the axis.
BENCH_PROSE = {"kth_successor": "$k$-th successor", "pointer_chase": "pointer chase"}

#: How each representation is named in prose.
ARM_PROSE = {"native": "native", "epserde": "$\\varepsilon$-serde", "rkyv": "rkyv"}

#: Text face of the target document, most-preferred first.
SERIF = ["Linux Libertine O", "Linux Libertine", "Libertinus Serif"]

#: OpenType files of those faces as TeX Live names them, used when the faces
#: are not installed system-wide but come with TeX (e.g., MacPorts, MacTeX).
TEX_FONTS = [
    "LinLibertine_R.otf", "LinLibertine_RI.otf",
    "LinLibertine_RB.otf", "LinLibertine_RBI.otf",
    "LibertinusSerif-Regular.otf", "LibertinusSerif-Italic.otf",
    "LibertinusSerif-Bold.otf", "LibertinusSerif-BoldItalic.otf",
]

#: acmsmall \textwidth: 6.75in paper less 46pt inner and 46pt outer margins.
ACMSMALL_TEXTWIDTH = 6.75 - 92 / 72.27

#: Default width: two figures side by side on a line, 0.2in apart.
HALF_WIDTH = (ACMSMALL_TEXTWIDTH - 0.2) / 2

#: Default bar width, as a fraction of the distance between the two bars.
BAR_WIDTH = 0.25

#: Bootstrap resamples for the confidence intervals.
RESAMPLES = 10_000


# ---------------------------------------------------------------- input
def load(source, baseline):
    """Per-benchmark per-iteration timings, keyed by `(group, arm, n)`.

    `source` is either a criterion directory or a `samples.json` mapping
    `group/arm/param` to the `times` and `iters` criterion recorded.
    """
    path = Path(source)
    if path.is_dir():
        obj = {}
        for p in sorted(path.glob(f"*/*/*/{baseline}/sample.json")):
            d = json.loads(p.read_text())
            obj["/".join(p.parts[-5:-2])] = d
        if not obj:
            present = sorted({q.name for q in path.glob("*/*/*/*") if q.is_dir()})
            sys.exit(f"error: no '{baseline}' baseline under {path}\n"
                     f"  baselines present: {', '.join(present) or 'none'}")
    elif path.is_file():
        obj = json.loads(path.read_text())
        if not isinstance(obj, dict):
            sys.exit(f"error: {path} is not a samples file (expected an object keyed "
                     f"`group/arm/param`)")
    else:
        sys.exit(f"error: no such file or directory: {path}\n"
                 f"  Produce one with:  extract_samples.py -o {path}")

    samples = {}
    for key, v in obj.items():
        group, arm, param = key.split("/")
        if group not in BENCHES or arm not in ARMS or not param.isdigit():
            continue
        samples[(group, arm, int(param))] = np.array(
            [t / i for t, i in zip(v["times"], v["iters"])])
    if not samples:
        sys.exit(f"error: {path} holds no recognisable benchmarks")
    return samples


def scale_label(n):
    """`n = 10^6` for powers of ten, `n = 2^20` for powers of two."""
    e = len(str(n)) - 1
    if n == 10 ** e and e > 0:
        return f"$n = 10^{{{e}}}$"
    if n and not (n & (n - 1)):
        return f"$n = 2^{{{n.bit_length() - 1}}}$"
    return f"$n = {n:,}$".replace(",", "\\,")


def size_of(samples, bench, subject, baseline):
    """The single node count at which both arms of `bench` were measured."""
    sizes = sorted(n for (g, a, n) in samples
                   if g == bench and a == subject and (g, baseline, n) in samples)
    if not sizes:
        sys.exit(f"error: the dataset has no {bench} measurement of both "
                 f"{subject} and {baseline}")
    if len(sizes) > 1:
        sys.exit(f"error: {bench} was measured at several sizes "
                 f"({', '.join(map(str, sizes))}); the figure has one bar per "
                 f"workload, so measure a single size")
    return sizes[0]


def overhead(samples, group, n, subject, baseline, rng):
    """Percentage by which `subject` is slower than `baseline`, with the
    bounds of a 95% bootstrap confidence interval."""
    s, b = samples[(group, subject, n)], samples[(group, baseline, n)]
    ratios = (rng.choice(s, (RESAMPLES, len(s))).mean(axis=1)
              / rng.choice(b, (RESAMPLES, len(b))).mean(axis=1) - 1) * 100
    lo, hi = np.percentile(ratios, [2.5, 97.5])
    return (s.mean() / b.mean() - 1) * 100, lo, hi


# ---------------------------------------------------------------- style
def tex_fonts():
    """Registers with matplotlib the files of TEX_FONTS that the TeX
    installation provides, as located by kpsewhich."""
    try:
        out = subprocess.run(["kpsewhich", *TEX_FONTS],
                             capture_output=True, text=True).stdout
    except OSError:  # no TeX installation
        return
    for path in out.splitlines():
        fm.fontManager.addfont(path)


def pick_serif(override):
    candidates = [override] if override else SERIF
    # Faces installed system-wide first, then those that come with TeX.
    for from_tex in (False, True):
        if from_tex:
            tex_fonts()
        available = {f.name for f in fm.fontManager.ttflist}
        for name in candidates:
            if name in available:
                return name
    sys.exit(
        f"error: none of {candidates} is available to matplotlib, either "
        f"installed or from TeX, so the figure would not match the paper.\n"
        f"  Fedora: sudo dnf install linux-libertine-fonts\n"
        f"  Debian: sudo apt install fonts-linuxlibertine\n"
        f"  then:   rm ~/.cache/matplotlib/fontlist-*.json\n"
        f"  or pass --font to name a face you do have."
    )


def style(serif, font_size):
    plt.rcParams.update({
        "font.family": "serif",
        "font.serif": [serif],
        # newtxmath is not available to matplotlib; setting the text face for
        # math too keeps the few mathematical labels consistent with the body.
        "mathtext.fontset": "custom",
        "mathtext.rm": serif,
        "mathtext.it": f"{serif}:italic",
        "mathtext.bf": f"{serif}:bold",
        # Unused here, but an unset cal face makes mathtext warn on every run.
        "mathtext.cal": f"{serif}:italic",
        "axes.unicode_minus": False,
        "pdf.fonttype": 42,
        "font.size": font_size,
        "axes.linewidth": 0.5,
        "xtick.major.width": 0.5, "ytick.major.width": 0.5,
        "xtick.major.size": 2.0, "ytick.major.size": 2.0,
        "xtick.direction": "out", "ytick.direction": "out",
    })


# ---------------------------------------------------------------- main
def main():
    here = Path(__file__).resolve().parent
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("source", nargs="?", default=str(here / "samples.json"),
                    help="a samples.json or a criterion directory "
                         "(default: samples.json here)")
    ap.add_argument("-o", "--out", default=str(here / "rkyv_overhead"),
                    help="output path without extension")
    ap.add_argument("--baseline", default="new",
                    help="criterion baseline to read from a directory (default: %(default)s)")
    ap.add_argument("--bench", action="append", choices=BENCHES, dest="benches",
                    help="restrict to these workloads (repeatable)")
    ap.add_argument("--subject", choices=ARMS, default="rkyv",
                    help="the representation whose cost is plotted")
    ap.add_argument("--baseline-arm", choices=ARMS, default="epserde", dest="ref",
                    help="the representation it is measured against")
    ap.add_argument("--width", type=float, default=HALF_WIDTH,
                    help=f"inches (default {HALF_WIDTH:.3f}, two figures on a line); "
                         f"acmsmall \\textwidth is {ACMSMALL_TEXTWIDTH:.3f}")
    ap.add_argument("--height", type=float, default=1.8, help="inches")
    ap.add_argument("--bar-width", type=float, default=BAR_WIDTH,
                    help="fraction of the distance between the bars (default: %(default)s)")
    ap.add_argument("--font-size", type=float, default=8.0,
                    help="points; 8 is \\footnotesize in a 10pt document")
    ap.add_argument("--font", help="text face to use instead of the paper's")
    ap.add_argument("--color", action="store_true")
    args = ap.parse_args()
    if args.subject == args.ref:
        sys.exit("error: --subject and --baseline-arm must differ")

    samples = load(args.source, args.baseline)
    benches = args.benches or [b for b in BENCHES if any(k[0] == b for k in samples)]
    if not benches:
        sys.exit("error: the dataset contains no kth_successor or pointer_chase measurements")
    sizes = [size_of(samples, b, args.subject, args.ref) for b in benches]

    serif = pick_serif(args.font)
    style(serif, args.font_size)
    fill = "#2a78d6" if args.color else "#a8a8a8"

    fig, ax = plt.subplots(figsize=(args.width, args.height))
    rng = np.random.default_rng(0)
    top, bottom = 0.0, 0.0

    for x, (bench, n) in enumerate(zip(benches, sizes)):
        m, lo, hi = overhead(samples, bench, n, args.subject, args.ref, rng)
        ax.bar(x, m, args.bar_width, color=fill, edgecolor="black", linewidth=0.5, zorder=3)
        ax.errorbar(x, m, yerr=[[m - lo], [hi - m]], fmt="none", ecolor="black",
                    elinewidth=0.5, capsize=1.5, capthick=0.5, zorder=4)
        below = m < 0
        ax.annotate(f"{m:.1f}", (x, lo if below else hi), textcoords="offset points",
                    xytext=(0, -2 if below else 2), ha="center",
                    va="top" if below else "baseline",
                    fontsize=args.font_size - 1.5, zorder=5)
        top, bottom = max(top, hi), min(bottom, lo)

    ax.set_xticks(range(len(benches)))
    ax.set_xticklabels([f"{BENCH_PROSE[b]}\n{scale_label(n)}"
                        for b, n in zip(benches, sizes)])
    ax.set_ylabel(f"{ARM_PROSE[args.subject]} overhead\nover {ARM_PROSE[args.ref]} (%)")
    span = top - bottom
    ax.set_ylim(bottom - 0.15 * span if bottom < 0 else 0,
                top + 0.15 * span if top > 0 else 0)
    if bottom < 0:
        ax.axhline(0, color="black", linewidth=0.5, zorder=3)
    ax.set_xlim(-0.6, len(benches) - 0.4)
    ax.yaxis.grid(True, color="#d8d8d8", linewidth=0.4, zorder=0)
    ax.set_axisbelow(True)
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    ax.tick_params(length=2.0, pad=2)
    ax.tick_params(axis="x", length=0)

    fig.tight_layout(pad=0.2)
    pdf, png = f"{args.out}.pdf", f"{args.out}.png"
    fig.savefig(pdf)
    fig.savefig(png, dpi=400)

    print(f"read   {args.source} ({len(samples)} measurements)")
    print(f"plot   {args.subject} against {args.ref}; "
          + ", ".join(f"{b} at {n} nodes" for b, n in zip(benches, sizes)))
    print(f"font   {serif} at {args.font_size}pt")
    print(f"size   {args.width:.3f} x {args.height:.3f} in")
    print(f"wrote  {pdf}\nwrote  {png}")


if __name__ == "__main__":
    main()
