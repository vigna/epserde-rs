#!/bin/sh
# Reproduces the Criterion results: timings for the native, ε-serde, and rkyv
# representations, for random k-th successor lookups (1 M nodes) and a
# pointer chase (1 000 nodes).
#
# Usage: ./reproduce.sh [CORE]   (default core 2; pick a performance core on
# hybrid CPUs, and keep the machine otherwise idle). The core is used only
# when taskset is available.
set -e
CORE=${1:-2}
DIR=${KTH_DIR:-target/data}
mkdir -p "$DIR"
export KTH_DIR="$DIR"

if command -v taskset >/dev/null 2>&1; then
    PIN="taskset -c $CORE"
    echo "== pinned to core $CORE"
else
    PIN=""
    echo "== taskset not available, not pinned"
fi
$PIN cargo bench --bench kth 2>&1 | grep -E "^(kth_successor|pointer_chase)| nodes, |time:   \[[0-9]"
rm -f "$DIR"/graph.epserde "$DIR"/graph.rkyv
