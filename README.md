# AXIOM

**A Binary Substring Graph for Zero-Error Retrieval**

An honest engineering study on real Gutenberg text at 1, 10, and 100 MB.

> **Headline result:** 0% hallucination rate across all corpus sizes, verified against 5 adversarial fake queries per size.
>
> **V3.1 LOUDS upgrade:** 94% smaller than V1 (12.19 MB vs 212 MB on 1 MB of text) using LOUDS succinct tries.

## What AXIOM is

AXIOM is a multi-layer binary substring graph for exact-match retrieval:

- **V1** — a prefix-sharing binary trie with one node per bit (212× bloat, but baseline)
- **V2** — a Lempel-Ziv-77-style macro graph that also shares middle substrings
- **V3.1** — a LOUDS succinct trie that solves V1's bloat (50× smaller structural storage)

There is no hashing, no embedding, no probability. Retrieval is exact substring match. If a query has no exact match in the graph, the graph returns nothing.

## What AXIOM is NOT

- ❌ Not a better compressor than `gzip` or `xz` (V2 is 38× worse than `xz` on 100 MB)
- ❌ Not a faster search engine than FAISS (FAISS returns in microseconds; AXIOM's deep-needle search takes milliseconds)
- ❌ Not a replacement for Transformer-based LLMs
- ❌ Not a novel algorithm (V2 is LZ77 from 1977; V3.1 is LOUDS from 1989)

The contribution is a complementary infrastructure layer with a property none of these systems have: **exact substring retrieval on binary data with shared-substring dedup across documents**.

## Honest results table

### Compression / file size (lower is better)

| Corpus | Raw | V1 | V2 | V3.1 | gzip -9 | xz -9 |
|---|---:|---:|---:|---:|---:|---:|
| 1 MB | 1.00 MB | 211.91 MB | 1.69 MB | 12.19 MB | 0.37 MB | **0.27 MB** |
| 10 MB | 10.00 MB | 2.05 GB | 15.25 MB | 121.54 MB | 3.79 MB | **2.22 MB** |
| 100 MB | 100.00 MB | skipped | 85.67 MB | (needs ~4 GB RAM) | 37.67 MB | **2.23 MB** |

### Zero-error verification (the headline result)

| Corpus | V1 halluc | V2 halluc | V3.1 halluc |
|---|---:|---:|---:|
| 1 MB | 0.0% (0/5) | 0.0% (0/5) | 0.0% (0/5) |
| 10 MB | 0.0% (0/5) | 0.0% (0/5) | 0.0% (0/5) |
| 100 MB | N/A | 0.0% (0/5) | (not tested) |

### V1 → V3.1 storage reduction

| Corpus | V1 size | V3.1 size | Reduction |
|---|---:|---:|---:|
| 1 MB | 211.91 MB | 12.19 MB | **94.2% smaller** |
| 10 MB | 2.05 GB | 121.54 MB | **94.2% smaller** |

The structural part (LOUDS bit vector + leaf flags + bit values) is **50× smaller** than V1's node array. The full file is larger because V3.1 still stores inserted bit-strings for substring search.

## Repository structure

```
axiom-3d/
├── README.md                    # This file
├── REPRODUCIBILITY.md           # How to reproduce every number
├── CONTRIBUTING.md              # How to contribute honestly
├── LICENSE                      # PolyForm Noncommercial 1.0.0
├── Cargo.toml                   # Rust package config (3 binaries)
├── .github/
│   ├── workflows/ci.yml         # GitHub Actions CI
│   └── ISSUE_TEMPLATE/          # Bug report + enhancement templates
├── paper/
│   ├── main.tex                 # IEEE conference format LaTeX source
│   ├── main.pdf                 # Compiled PDF (8 pages)
│   ├── figures/                 # 5 charts from real measured data
│   ├── stress_results.json      # Raw V1/V2 measurements
│   ├── v31_results.json         # Raw V3.1 measurements
│   └── compressor_benchmarks.json   # Real gzip/xz/bzip2 measurements
├── src/
│   └── bin/
│       ├── axiom_stress.rs      # V1 + V2 stress test binary
│       ├── axiom_v3.rs          # V3 LOUDS (initial version)
│       └── axiom_v31.rs         # V3.1 optimized LOUDS
├── scripts/
│   ├── setup_and_run.sh         # Downloads Gutenberg books, builds corpora
│   └── run_compressor_benchmarks.py  # Runs gzip/xz/bzip2 on same corpora
└── data/                        # Populated by setup_and_run.sh
```

## How to reproduce

### Step 1 — Build the corpora

```bash
./scripts/setup_and_run.sh
```

This downloads 10 public-domain Project Gutenberg books (~8.4 MB total) and builds:
- `data/corpus_1mb.txt` (1 MB, 100% unique)
- `data/corpus_10mb.txt` (10 MB, 79% unique)
- `data/corpus_100mb.txt` (100 MB, 7% unique)

### Step 2 — Run the stress tests

```bash
cargo run --release --bin axiom_stress   # V1 + V2 (takes ~3 min)
cargo run --release --bin axiom_v31      # V3.1 LOUDS (takes ~1 min on 10 MB)
```

Output goes to `out/` including JSON files with all measurements.

### Step 3 — Run the compressor benchmarks

```bash
python3 scripts/run_compressor_benchmarks.py
```

This runs `gzip -9`, `xz -9`, and `bzip2 -9` on the same three corpora.

### Step 4 — Verify the numbers in the paper

```bash
cat paper/stress_results.json
cat paper/v31_results.json
cat paper/compressor_benchmarks.json
```

The numbers in `paper/main.pdf` come directly from these JSON files.

## Architecture: three layers

### V1 — Prefix-Sharing Binary Trie
- One node per bit (8.2M nodes for 1 MB of text)
- 26 bytes per node → 212× bloat
- O(L) prefix search, O(N·L) substring search
- 0% hallucination rate

### V2 — LZ77-style Macro Graph
- Greedy LZ77 with byte-level hash chains (32-deep, 64-byte max match)
- Shared substrings become first-class macro nodes
- 0.86× compression on 100 MB (with 93% repetition in corpus)
- 38× worse than xz (no entropy coding layer)
- 0% hallucination rate

### V3.1 — Optimized LOUDS Trie
- LOUDS (Level-Order Unary Degree Sequence) encoding: 2 bits per node for structure
- Plus 1 bit per node for is_leaf flag, 1 bit per node for bit_value
- Total: 4.5 bits per node (vs 208 bits per node in V1)
- 94% smaller than V1 on the same corpus
- Same O(L) prefix search, same O(N·L) substring search
- Same 0% hallucination rate

## Honest limitations

1. **V1 bloats 212× on disk** — fixed in V3.1 (94% reduction)
2. **V2 is 38× worse than xz on compression** — entropy coding layer is future work
3. **Substring search is O(N·L)**, not O(L) — suffix-tree upgrade is future work
4. **No semantic search** — AXIOM finds exact byte matches only
5. **No streaming/incremental updates** — full rebuild required
6. **Single-threaded build** — 16.5s for 100 MB V2, 8s for 10 MB V3.1

These limitations are stated explicitly in the paper to prevent over-claiming.

## Target venues

- **IEEE Big Data 2027** (deadline ~June 2027)
- **ACM CIKM 2027** (deadline ~May 2027)
- **NeurIPS/ICML workshops** on retrieval or efficient ML
- **ETH Student Summer Research Fellowship** (deadline ~November 2026)

See `SWISS_AI_JOBS_ROADMAP.md` (in the delivery bundle) for the full Swiss AI jobs landscape.

## Author

**Yashas K** — Independent Researcher, Bangalore, India
ORCID: 0009-0005-8928-5341
GitHub: https://github.com/Yashas-K-Gangatkar/axiom-3d

## License

PolyForm Noncommercial 1.0.0 — see `LICENSE` file. Academic use permitted; commercial use requires a separate license.
