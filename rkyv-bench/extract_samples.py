#!/usr/bin/env python3
"""Extract raw timings from a criterion tree into a single `samples.json`.

    ./extract_samples.py target/criterion -o samples.json

criterion writes its results under `target/criterion`, which is not checked in
and does not survive a `cargo clean`. This pulls out every individual timing
and writes them to one file, which is what the figure is drawn from.

The output maps `group/arm/param` to the `times` and `iters` criterion
recorded, e.g.

    {"pointer_chase/rkyv/1000": {"iters": [...], "times": [...], ...}}

Dividing `times` by `iters` elementwise gives nanoseconds per iteration.
"""

import argparse
import json
import sys
from pathlib import Path


def main():
    here = Path(__file__).resolve().parent
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("tree", nargs="?", default=str(here / "target" / "criterion"),
                    help="the criterion directory (default: target/criterion here)")
    ap.add_argument("--baseline", default="new",
                    help="which saved baseline to read (default: %(default)s)")
    ap.add_argument("-o", "--out", default=str(here / "samples.json"),
                    help="output file (default: samples.json here)")
    args = ap.parse_args()

    tree = Path(args.tree)
    if not tree.is_dir():
        sys.exit(f"error: no such criterion directory: {tree}")

    out = {}
    for p in sorted(tree.glob(f"*/*/*/{args.baseline}/sample.json")):
        key = "/".join(p.parts[-5:-2])          # group/arm/param
        d = json.loads(p.read_text())
        out[key] = {"iters": d["iters"], "times": d["times"],
                    "sampling_mode": d.get("sampling_mode")}

    if not out:
        present = sorted({q.name for q in tree.glob("*/*/*/*") if q.is_dir()})
        sys.exit(f"error: no '{args.baseline}' baseline under {tree}\n"
                 f"  baselines present: {', '.join(present) or 'none'}")

    dst = Path(args.out)
    dst.write_text(json.dumps(out, separators=(",", ":")))
    timings = sum(len(v["times"]) for v in out.values())
    print(f"read   {tree} (baseline '{args.baseline}')")
    print(f"wrote  {dst}: {len(out)} benchmarks, {timings:,} timings, "
          f"{dst.stat().st_size:,} bytes")


if __name__ == "__main__":
    main()
