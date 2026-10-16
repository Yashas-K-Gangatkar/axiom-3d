# AXIOM

**A Binary Substring Graph for Zero-Error Retrieval**

An honest engineering study on real Gutenberg text at 1, 10, and 100 MB.

> **Headline result:** 0% hallucination rate across all corpus sizes, verified against 5 adversarial fake queries per size.
>
> **Honest limitation:** AXIOM V2 is 38× worse than `xz -9` on compression ratio. V1 bloats 212× on disk. Substring search is O(N·L), not O(L). These are stated explicitly in the paper.

## What AXIOM is

AXIOM is a two-layer binary substring graph for exact-match retrieval:

- **V1** — a prefix-sharing binary trie with one node per bit
- **V2** — a Lempel-Ziv-77-style macro graph that also shares middle substrings

There is no hashing, no embedding, no probability. Retrieval is exact substring match. If a query has no exact match in the graph, the graph returns nothing.

## What AXIOM is NOT

- ❌ Not a better compressor than `gzip` or `xz` (V2 is 38× worse than `xz` on 100 MB)
- ❌ Not a faster search engine than FAISS (FAISS returns in microseconds; AXIOM's deep-needle search takes milliseconds)
- ❌ Not a replacement for Transformer-based LLMs
- ❌ Not a novel algorithm (V2 is literally LZ77)

The contribution is a complementary infrastructure layer with a property none of these systems have: **exact substring retrieval on binary data with shared-substring dedup across documents**.

## Repository structure

```
axiom/
├── README.md                    # This file
├── LICENSE                      PolyForm Noncommercial 1.0.0
├── paper/
│   ├── main.tex                 IEEE conference format LaTeX source
│   ├── main.pdf                 Compiled PDF (8 pages)
│   ├── figures/                 5 charts generated from real measured data
│   ├── stress_results.json      Raw AXIOM V1/V2 measurements
│   └── compressor_benchmarks.json   Real gzip/xz/bzip2 measurements on same corpora
├── src/
│   └── bin/
│       └── axiom_stress.rs      Rust stress-test binary (single file, no deps)
├── scripts/
│   ├── setup_and_run.sh         Downloads Gutenberg books, builds 1/10/100 MB corpora
│   └── run_compressor_benchmarks.py  Runs gzip/xz/bzip2 on same corpora
└── data/                        (populated by setup_and_run.sh)
```

## How to reproduce

### Step 1 — Download corpora

```bash
./scripts/setup_and_run.sh
```

This downloads 10 public-domain Project Gutenberg books (~8.4 MB total) and builds:
- `data/corpus_1mb.txt` (1 MB, 100% unique)
- `data/corpus_10mb.txt` (10 MB, 79% unique)
- `data/corpus_100mb.txt` (100 MB, 7% unique)

### Step 2 — Run the AXIOM stress test

You need Rust 1.99 or newer. From this repo's root:

```bash
cargo run --release --bin axiom_stress
```

This builds V1 and V2 graphs for all three corpora, runs:
- Needle-in-haystack test (first 80 bytes)
- Deep needle test (80 bytes from middle)
- Hallucination test (5 adversarial fake queries per corpus)

Output goes to `out/` including `stress_results.json`.

### Step 3 — Run the compressor benchmarks

```bash
python3 scripts/run_compressor_benchmarks.py
```

This runs `gzip -9`, `xz -9`, and `bzip2 -9` on the same three corpora and writes `compressor_benchmarks.json`.

### Step 4 — Verify the numbers in the paper

```bash
cat paper/stress_results.json
cat paper/compressor_benchmarks.json
```

The numbers in `paper/main.pdf` come directly from these two JSON files. If you regenerate the corpora and get different numbers, the paper's numbers will be wrong — that's why the JSON is included.

## Honest results table

| Corpus | Raw | AXIOM V1 | AXIOM V2 | gzip -9 | bzip2 -9 | xz -9 |
|---|---:|---:|---:|---:|---:|---:|
| 1 MB | 1.00 MB | 211.91 MB | 1.69 MB | 0.37 MB | 0.27 MB | **0.27 MB** |
| 10 MB | 10.00 MB | 2.05 GB | 15.25 MB | 3.79 MB | 2.80 MB | **2.22 MB** |
| 100 MB | 100.00 MB | skipped | 85.67 MB | 37.67 MB | 28.22 MB | **2.23 MB** |

| Corpus | Hallucination rate (V2) |
|---|---:|
| 1 MB | 0.0% (0/5 false positives) |
| 10 MB | 0.0% (0/5 false positives) |
| 100 MB | 0.0% (0/5 false positives) |

## Target venues

This paper is structured for submission to:
- IEEE Big Data 2027 (deadline ~June 2027)
- ACM CIKM 2027 (deadline ~May 2027)
- NeurIPS/ICML workshops on retrieval or efficient ML (deadlines ~Aug/Sep 2027)

No arXiv endorsement is required for any of these venues.

## Author

**Yashas K** — Independent Researcher, Bangalore, India
ORCID: 0009-0005-8928-5341

## License

PolyForm Noncommercial 1.0.0 — see `LICENSE` file. Academic use is permitted; commercial use requires a separate license.
