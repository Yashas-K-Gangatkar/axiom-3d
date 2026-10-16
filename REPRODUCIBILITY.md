# Reproducibility Guide

This document describes exactly how to reproduce every number in the AXIOM paper. Every step is deterministic given the same hardware and Rust version.

## Hardware requirements

- **RAM**: 16 GB minimum (V1 on 100 MB requires ~20 GB and is skipped by default)
- **Disk**: 2 GB free for corpora and output binaries
- **CPU**: Any modern x86_64 or ARM64; single-threaded benchmarks
- **OS**: Linux, macOS, or WSL2 on Windows

## Software requirements

- **Rust**: 1.99 or newer (`rustup install stable`)
- **Python**: 3.8+ with `matplotlib` and `pypdf` (only for chart generation)
- **Standard compressors**: `gzip`, `xz`, `bzip2` (pre-installed on Linux/macOS)
- **Tectonic** (for compiling the paper): https://tectonic-typesetting.github.io/

## Step-by-step reproduction

### Step 1: Clone and build

```bash
git clone https://github.com/Yashas-K-Gangatkar/axiom-3d.git
cd axiom-3d
cargo build --release
```

This produces three binaries:
- `axiom_stress` — V1 prefix trie + V2 LZ77 stress test
- `axiom_v3` — V3 LOUDS trie (initial version)
- `axiom_v31` — V3.1 optimized LOUDS trie

### Step 2: Build the corpora

```bash
./scripts/setup_and_run.sh
```

This downloads 10 public-domain Project Gutenberg books (~8.4 MB total) and builds:
- `data/corpus_1mb.txt` (1 MB, 100% unique content)
- `data/corpus_10mb.txt` (10 MB, 79% unique content)
- `data/corpus_100mb.txt` (100 MB, 7% unique content, 93% repetition)

The 10/100 MB corpora intentionally contain repetition because real-world corpora (web crawls, codebases, document archives) do too. Repetition is what AXIOM V2's shared-substring design exploits.

### Step 3: Run the AXIOM stress tests

```bash
# V1 + V2 stress test
cargo run --release --bin axiom_stress

# V3 LOUDS (will take ~5-30 minutes for 100 MB depending on hardware)
cargo run --release --bin axiom_v3

# V3.1 optimized LOUDS
cargo run --release --bin axiom_v31
```

Each binary writes:
- Binary graph files to `out/*.bin`
- JSON results to `out/stress_results.json`, `out/v3_results.json`, `out/v31_results.json`
- Run log to `stdout` (capture with `| tee out/run.log` if you want a copy)

### Step 4: Run the compressor benchmarks

```bash
python3 scripts/run_compressor_benchmarks.py
```

This runs `gzip -9`, `xz -9`, and `bzip2 -9` on the same three corpora and writes `out/compressor_benchmarks.json`.

### Step 5: Verify the numbers match the paper

The numbers in `paper/main.pdf` come from these JSON files:

| Paper table | Source JSON |
|---|---|
| Table I (on-disk file size) | `out/stress_results.json` (V1/V2) + `out/compressor_benchmarks.json` (gzip/xz/bzip2) |
| Table II (compression ratio) | Same as Table I |
| Table III (build time) | Same + V3/V3.1 results |
| Table IV (search time) | `out/stress_results.json` |
| Table V (hallucination) | `out/stress_results.json` |

If your numbers differ from the paper, possible reasons:
1. **Different Rust version** — should give identical numbers
2. **Different CPU** — search times will vary by ±20%
3. **Different Gutenberg books downloaded** — file sizes will vary slightly
4. **You regenerated the corpora** — make sure you used `setup_and_run.sh`

### Step 6: Regenerate the charts

If you want to regenerate the charts with your own numbers:

```bash
python3 scripts/generate_paper_charts_v2.py
```

This reads `out/stress_results.json` and `out/compressor_benchmarks.json` and writes 5 PNG files to `paper/figures/`.

### Step 7: Recompile the paper

If you changed the LaTeX source or charts:

```bash
cd paper
tectonic main.tex
```

Output: `paper/main.pdf`

## Expected results (your numbers should be within ±20%)

### V1 + V2 stress test

| Corpus | Raw | V1 file | V2 file | V2/raw ratio |
|---|---:|---:|---:|---:|
| 1 MB | 1.00 MB | 211.91 MB | 1.69 MB | 1.69× |
| 10 MB | 10.00 MB | 2.05 GB | 15.25 MB | 1.53× |
| 100 MB | 100.00 MB | skipped | 85.67 MB | 0.86× |

### Compressor benchmarks

| Corpus | gzip -9 | xz -9 | bzip2 -9 |
|---|---:|---:|---:|
| 1 MB | 0.37 MB | 0.27 MB | 0.27 MB |
| 10 MB | 3.79 MB | 2.22 MB | 2.80 MB |
| 100 MB | 37.67 MB | 2.23 MB | 28.22 MB |

### V3.1 LOUDS

| Corpus | V3.1 structural | V3.1 file | V3.1/raw ratio | Hallucination |
|---|---:|---:|---:|---:|
| 1 MB | 4.41 MB | 12.19 MB | 12.19× | 0.0% |
| 10 MB | 43.78 MB | 121.54 MB | 12.15× | 0.0% |
| 100 MB | (would need ~4 GB RAM, not tested) | — | — | — |

## What "honest" means in this context

Every number in this repository is directly measured. None are:
- Estimated from theory
- Extrapolated from smaller tests
- Copied from literature
- Rounded favorably
- Cherry-picked from multiple runs

If a measurement fails (e.g., V1 on 100 MB requires too much RAM), we say so explicitly rather than skipping it silently. If a benchmark would be unfair (e.g., comparing AXIOM to FAISS as if they solve the same problem), we don't make the comparison.

## If you find a bug

If you find a bug that affects the numbers:
1. Open an issue at https://github.com/Yashas-K-Gangatkar/axiom-3d/issues
2. Include the JSON output file showing the wrong number
3. Include your Rust version and OS

If the bug invalidates a paper claim, the paper will be updated and re-released with a correction notice. Academic honesty is more important than protecting the paper's claims.
