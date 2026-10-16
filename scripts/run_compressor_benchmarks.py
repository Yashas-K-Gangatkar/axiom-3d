#!/usr/bin/env python3
"""Real benchmark of standard compressors on the same 1/10/100MB corpora
that AXIOM was tested on. Output goes to compressor_benchmarks.json
with actual measured file sizes and compression times.
NO FAKE DATA. Every number is real."""

import json
import os
import subprocess
import time
import tempfile

DATA_DIR = "/home/z/my-project/scripts/yk_stress/data"
OUT_DIR = "/home/z/my-project/scripts/yk_stress/out"
os.makedirs(OUT_DIR, exist_ok=True)

CORPORA = ["corpus_1mb", "corpus_10mb", "corpus_100mb"]
COMPRESSORS = ["gzip", "xz", "bzip2"]

results = []

for corpus in CORPORA:
    src_path = f"{DATA_DIR}/{corpus}.txt"
    if not os.path.exists(src_path):
        print(f"  SKIP {corpus} (file missing)")
        continue
    src_size = os.path.getsize(src_path)
    with open(src_path, "rb") as f:
        src_data = f.read()
    print(f"\n=== {corpus} ({src_size:,} bytes) ===")
    
    for tool in COMPRESSORS:
        flags = ["-9", "-c"]
        # bzip2 reads from stdin with -c
        t0 = time.perf_counter()
        try:
            proc = subprocess.run(
                [tool] + flags,
                input=src_data,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=True,
            )
            compressed = proc.stdout
            t1 = time.perf_counter()
            elapsed = t1 - t0
            out_size = len(compressed)
            ratio = out_size / src_size if src_size > 0 else 0
            
            print(f"  {tool:8s}  src={src_size:>12,}  out={out_size:>12,}  "
                  f"ratio={ratio:.4f}  time={elapsed:.3f}s")
            
            results.append({
                "corpus": corpus,
                "compressor": tool,
                "src_bytes": src_size,
                "out_bytes": out_size,
                "ratio": round(ratio, 6),
                "time_s": round(elapsed, 4),
            })
        except subprocess.CalledProcessError as e:
            print(f"  {tool:8s}  FAILED: {e.stderr.decode()[:200]}")
            results.append({
                "corpus": corpus,
                "compressor": tool,
                "src_bytes": src_size,
                "out_bytes": None,
                "ratio": None,
                "time_s": None,
                "error": e.stderr.decode()[:200],
            })

# Also include the AXIOM V2 numbers for direct comparison
# (these are from the real stress test, already measured)
axiom_v2 = {
    "corpus_1mb":   {"out_bytes": 1768963,   "ratio": 1.6870, "build_s": 0.083},
    "corpus_10mb":  {"out_bytes": 15991464,  "ratio": 1.5251, "build_s": 1.49},
    "corpus_100mb": {"out_bytes": 89827628,  "ratio": 0.8567, "build_s": 16.49},
}

output = {
    "compressor_benchmarks": results,
    "axiom_v2_for_comparison": axiom_v2,
    "methodology": {
        "compressors_tested": COMPRESSORS,
        "compressor_flags": "-9 -c (max compression, stdout)",
        "corpora": CORPORA,
        "corpus_source": "Project Gutenberg public-domain books, concatenated",
        "corpus_unique_bytes": 8361920,
        "corpus_repetition_1mb": "100% unique (subset of unique books)",
        "corpus_repetition_10mb": "79% unique (full 8.36MB books repeated to 10MB)",
        "corpus_repetition_100mb": "7% unique (books repeated 12x to 100MB)",
        "timing_method": "Python time.perf_counter() around subprocess.run()",
        "size_method": "len(compressed_bytes) from subprocess stdout",
        "platform": "Linux, single-threaded",
    }
}

out_path = f"{OUT_DIR}/compressor_benchmarks.json"
with open(out_path, "w") as f:
    json.dump(output, f, indent=2)
print(f"\nResults written to {out_path}")
print(f"\n=== Summary table ===")
print(f"{'Corpus':<15} {'Tool':<10} {'Out bytes':>14} {'Ratio':>8} {'Time(s)':>10}")
print("-" * 60)
for r in results:
    print(f"{r['corpus']:<15} {r['compressor']:<10} {r['out_bytes']:>14,} {r['ratio']:>8.4f} {r['time_s']:>10.3f}")
print("-" * 60)
for corpus, data in axiom_v2.items():
    print(f"{corpus:<15} {'AXIOM V2':<10} {data['out_bytes']:>14,} {data['ratio']:>8.4f} {data['build_s']:>10.3f}")
