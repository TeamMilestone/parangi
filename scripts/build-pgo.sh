#!/bin/bash
# Build parang with Profile-Guided Optimization (PGO)
# Usage: ./scripts/build-pgo.sh [training_file_list]
# Default training list: /tmp/parang-bench-100.txt
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

TRAIN_LIST="${1:-/tmp/parang-bench-100.txt}"
PGO_DIR="/tmp/pgo-data"
PGO_MERGED="/tmp/pgo-merged.profdata"
PROFDATA="$(find "$HOME/.rustup" -name llvm-profdata -type f 2>/dev/null | head -1)"
if [ -z "$PROFDATA" ]; then
    echo "Error: llvm-profdata not found. Install llvm-tools: rustup component add llvm-tools"
    exit 1
fi

echo "=== PGO Build Pipeline ==="
echo "Training list: $TRAIN_LIST"
echo "Profile data dir: $PGO_DIR"

# Step 1: Instrumented build
echo ""
echo "Step 1/4: Building instrumented binary..."
rm -rf "$PGO_DIR"
RUSTFLAGS="-Cprofile-generate=$PGO_DIR" cargo build --release

# Step 2: Collect profiles
echo ""
echo "Step 2/4: Collecting profiles (3 runs)..."
for i in 1 2 3; do
    echo "  Run $i/3..."
    RAYON_NUM_THREADS=1 target/release/parang -L "$TRAIN_LIST" > /dev/null 2>&1 || true
done

# Step 3: Merge profiles
echo ""
echo "Step 3/4: Merging profiles..."
"$PROFDATA" merge -o "$PGO_MERGED" "$PGO_DIR/"
echo "  Merged: $(ls -lh "$PGO_MERGED" | awk '{print $5}')"

# Step 4: Optimized build
echo ""
echo "Step 4/4: Building PGO-optimized binary..."
RUSTFLAGS="-Cprofile-use=$PGO_MERGED" cargo build --release

echo ""
echo "=== PGO build complete ==="
echo "Binary: target/release/parang"
