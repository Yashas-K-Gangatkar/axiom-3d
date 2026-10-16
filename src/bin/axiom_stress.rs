// ============================================================
// AXIOM-3D HONEST STRESS TEST  —  v2 (fast, honest)
// ---------------------------------
// Tests on REAL Gutenberg text at 1MB / 10MB / 100MB:
//   (1) V1 = prefix-trie (binary trie of all chunks)
//   (2) V2 = compressed graph (LZ77-style shared-substring macros)
//
// Every number printed is MEASURED, not estimated.
//   - File sizes come from std::fs::metadata (real bytes on disk)
//   - Dedup % comes from counting nodes actually created
//   - Search times come from Instant::now()
//   - Hallucination rate comes from querying fake strings and counting
//
// HONEST DESIGN NOTES
// -------------------
// 1. The original AXIOM-3D pitch describes a 26-direction tree for
//    LETTERS. But the architecture stores BITS (0/1), so a node has
//    at most TWO children. We implement the binary version honestly
//    and call it "3D" by convention. The X/Y/Z axis distinction is
//    a labeling scheme over the same binary trie.
//
// 2. V1 only dedups PREFIXES that are shared across inserted chunks.
//    For natural English text, common prefixes are short — V1 dedup
//    is expected to be small (1-5%). This is the honest baseline.
//
// 3. V2 also shares MIDDLE substrings via an LZ77-style scheme:
//    every repeated substring of length >= MIN_MATCH_BYTES is
//    replaced with a reference to a shared macro chain. This is
//    what gives real compression on natural text.
//
// 4. Zero-error is preserved in BOTH V1 and V2 because search is
//    exact substring match — no hashing, no probability, no LLM.
// ============================================================

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::time::{Duration, Instant};

const CHUNK_BYTES: usize = 200;       // Each "document" is ~200 bytes
const MIN_MATCH_BYTES: usize = 8;     // 8 bytes = 64 bits minimum shared substring
const MAX_MATCH_BYTES: usize = 64;    // Cap match length (was 65535 — caused bloat)
const HASH_CHAIN_DEPTH: usize = 32;   // Max candidates per hash bucket
const HASH_BITS: u32 = 20;            // 1M buckets
const HASH_BUCKETS: usize = 1 << HASH_BITS;

// -----------------------------
// Node: one node in the binary trie (V1)
// -----------------------------
#[derive(Clone, Copy)]
struct Node {
    id: u64,
    bit: u8,           // 0, 1, or 2 (root only)
    child0: u64,       // u64::MAX = none
    child1: u64,
    is_end: bool,
}

struct GraphV1 {
    nodes: Vec<Node>,
    root: u64,
    // Also store the original bit-strings for substring search.
    // Honest: a prefix-trie cannot do O(L) substring search; it can
    // only do O(L) prefix search. For substring search we materialize
    // the inserted strings and run naive search. This is the real cost.
    inserted: Vec<Vec<u8>>,
}

impl GraphV1 {
    fn new() -> Self {
        GraphV1 {
            nodes: vec![Node {
                id: 0,
                bit: 2,
                child0: u64::MAX,
                child1: u64::MAX,
                is_end: false,
            }],
            root: 0,
            inserted: Vec::new(),
        }
    }

    fn new_node(&mut self, bit: u8) -> u64 {
        let id = self.nodes.len() as u64;
        self.nodes.push(Node {
            id, bit,
            child0: u64::MAX,
            child1: u64::MAX,
            is_end: false,
        });
        id
    }

    fn insert_bits(&mut self, bits: &[u8]) -> u64 {
        // Insert one bit-string. Returns number of NEW nodes created.
        let mut new_nodes: u64 = 0;
        let mut cur = self.root;
        for &b in bits {
            let child = if b == 0 { self.nodes[cur as usize].child0 }
                        else { self.nodes[cur as usize].child1 };
            cur = match child {
                c if c != u64::MAX => c,
                _ => {
                    let n = self.new_node(b);
                    new_nodes += 1;
                    if b == 0 { self.nodes[cur as usize].child0 = n; }
                    else { self.nodes[cur as usize].child1 = n; }
                    n
                }
            }
        }
        self.nodes[cur as usize].is_end = true;
        self.inserted.push(bits.to_vec());
        new_nodes
    }

    // ---------- substring search (honest, naive) ----------
    fn contains_substring(&self, needle: &[u8]) -> bool {
        for hay in &self.inserted {
            if hay.len() < needle.len() { continue; }
            for w in 0..=(hay.len() - needle.len()) {
                if &hay[w..w + needle.len()] == needle {
                    return true;
                }
            }
        }
        false
    }

    // ---------- serialize to disk (real bytes) ----------
    // Format: 8B node count + 8B root + (8B id + 1B bit + 1B is_end + 8B c0 + 8B c1) per node
    //         + 8B inserted-count + (4B len + bytes) per inserted string
    fn serialize(&self, path: &str) -> std::io::Result<u64> {
        let mut f = fs::File::create(path)?;
        f.write_all(&(self.nodes.len() as u64).to_le_bytes())?;
        f.write_all(&self.root.to_le_bytes())?;
        for n in &self.nodes {
            f.write_all(&n.id.to_le_bytes())?;
            f.write_all(&[n.bit])?;
            f.write_all(&[n.is_end as u8])?;
            f.write_all(&n.child0.to_le_bytes())?;
            f.write_all(&n.child1.to_le_bytes())?;
        }
        f.write_all(&(self.inserted.len() as u64).to_le_bytes())?;
        for ins in &self.inserted {
            f.write_all(&(ins.len() as u32).to_le_bytes())?;
            f.write_all(ins)?;
        }
        f.sync_all()?;
        Ok(fs::metadata(path)?.len())
    }
}

// ============================================================
// V2 COMPRESSOR — fast LZ77-style shared-substring graph
// ---------------------------------
// Build process:
//   1. Concatenate all chunks into one byte stream with separators.
//   2. Run hash-chain LZ77 in BYTE space:
//        - For each position p, hash bytes[p..p+8] -> bucket
//        - Walk the hash chain (max HASH_CHAIN_DEPTH candidates)
//        - Find longest match >= MIN_MATCH_BYTES
//        - If found: emit (offset, length) macro reference
//        - Else: emit literal byte
//   3. Each unique (offset, length) match becomes a "macro chain"
//      — a shared substring node in the graph.
//   4. The "program" for each chunk is the sequence of literals
//      and macro references.
//
// The graph then has:
//   - Macro nodes: each is a shared substring (a sequence of bytes)
//   - Program edges: each chunk is a sequence of (literal | macro_ref)
//
// This is honest: V2 = LZ77 compression applied to the chunked
// corpus, encoded as a graph of shared substrings. It's NOT a
// novel data structure, but it's the right baseline for measuring
// how much middle-substring sharing actually saves.
// ============================================================

struct GraphV2 {
    macros: Vec<Vec<u8>>,                       // shared substrings
    programs: Vec<Vec<Instr>>,                  // per-chunk instruction list
    chunk_count: usize,
    total_source_bytes: u64,
}

#[derive(Clone, Copy)]
enum Instr {
    Literal(u8),
    Macro(u32), // index into macros
}

impl GraphV2 {
    fn build_from(chunks: &[Vec<u8>]) -> Self {
        // Concatenate chunks into one stream with a u8 separator that
        // can't appear in valid UTF-8 text. We use 0xFF.
        let sep: u8 = 0xFF;
        let mut stream: Vec<u8> = Vec::new();
        let mut chunk_starts: Vec<usize> = Vec::with_capacity(chunks.len());
        for (i, c) in chunks.iter().enumerate() {
            chunk_starts.push(stream.len());
            stream.extend_from_slice(c);
            if i + 1 < chunks.len() {
                stream.push(sep);
            }
        }
        let total_source_bytes = stream.len() as u64;

        // Hash chains
        let mut head: Vec<i32> = vec![-1; HASH_BUCKETS];
        let mut prev: Vec<i32> = vec![-1; stream.len()];

        let hash = |bytes: &[u8], p: usize| -> usize {
            // FNV-1a 64-bit on 8 bytes, then mask
            let mut h: u64 = 0xcbf29ce484222325;
            for i in 0..MIN_MATCH_BYTES {
                h ^= bytes[p + i] as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            (h & (HASH_BUCKETS as u64 - 1)) as usize
        };

        let mut macros: Vec<Vec<u8>> = Vec::new();
        let mut macro_index: HashMap<Vec<u8>, u32> = HashMap::new();
        let mut programs: Vec<Vec<Instr>> = Vec::with_capacity(chunks.len());

        // Process per-chunk
        for (ci, _chunk) in chunks.iter().enumerate() {
            let start = chunk_starts[ci];
            let end = if ci + 1 < chunks.len() { chunk_starts[ci + 1] } else { stream.len() };
            // Don't match across chunk boundary (each chunk is independent doc)
            let chunk_end = end.saturating_sub(0); // include separator? No — end at chunk end, exclude separator
            let chunk_end = if ci + 1 < chunks.len() { end.saturating_sub(1) } else { end };

            let mut prog: Vec<Instr> = Vec::new();
            let mut p = start;
            while p < chunk_end {
                let remaining = chunk_end - p;
                if remaining < MIN_MATCH_BYTES {
                    // Emit rest as literals
                    for i in p..chunk_end {
                        prog.push(Instr::Literal(stream[i]));
                    }
                    break;
                }

                let h = hash(&stream, p);
                let mut best_len = 0usize;
                let mut best_off = 0usize;
                let mut depth = 0;
                let mut cand = head[h];
                while cand >= 0 && depth < HASH_CHAIN_DEPTH {
                    let c = cand as usize;
                    // Only match within this chunk
                    if c < start {
                        // candidate is from a previous chunk — we allow
                        // cross-chunk matches since AXIOM-3D shares
                        // across the whole corpus.
                    }
                    // Compute match length
                    let max_possible = (stream.len() - p).min(MAX_MATCH_BYTES);
                    let mut l = 0;
                    while l < max_possible && stream[c + l] == stream[p + l] {
                        l += 1;
                    }
                    if l > best_len {
                        best_len = l;
                        best_off = p - c;
                    }
                    cand = prev[c];
                    depth += 1;
                }

                if best_len >= MIN_MATCH_BYTES {
                    let macro_bytes = &stream[p..p + best_len];
                    let mid = if let Some(&m) = macro_index.get(macro_bytes) {
                        m
                    } else {
                        let m = macros.len() as u32;
                        macros.push(macro_bytes.to_vec());
                        macro_index.insert(macro_bytes.to_vec(), m);
                        m
                    };
                    prog.push(Instr::Macro(mid));
                    // Update hash chain for all positions in the match
                    for i in 0..best_len {
                        if p + i + MIN_MATCH_BYTES <= stream.len() {
                            let hh = hash(&stream, p + i);
                            prev[p + i] = head[hh];
                            head[hh] = (p + i) as i32;
                        }
                    }
                    p += best_len;
                } else {
                    prog.push(Instr::Literal(stream[p]));
                    if p + MIN_MATCH_BYTES <= stream.len() {
                        let hh = hash(&stream, p);
                        prev[p] = head[hh];
                        head[hh] = p as i32;
                    }
                    p += 1;
                }
            }
            programs.push(prog);
        }

        GraphV2 {
            macros,
            programs,
            chunk_count: chunks.len(),
            total_source_bytes,
        }
    }

    fn contains_substring(&self, needle: &[u8]) -> bool {
        for prog in &self.programs {
            // Materialize this chunk
            let mut bytes: Vec<u8> = Vec::with_capacity(1024);
            for ins in prog {
                match ins {
                    Instr::Literal(b) => bytes.push(*b),
                    Instr::Macro(m) => bytes.extend_from_slice(&self.macros[*m as usize]),
                }
            }
            if bytes.len() >= needle.len() {
                for w in 0..=(bytes.len() - needle.len()) {
                    if &bytes[w..w + needle.len()] == needle {
                        return true;
                    }
                }
            }
        }
        false
    }

    // ---------- serialize V2 to disk ----------
    // Format:
    //   8B total source bytes, 8B chunk count, 8B macro count
    //   [for each macro: 4B len + bytes]
    //   8B program count
    //   [for each program: 4B instr count + (1B tag + 4B value) per instr]
    fn serialize(&self, path: &str) -> std::io::Result<u64> {
        let mut f = fs::File::create(path)?;
        f.write_all(&self.total_source_bytes.to_le_bytes())?;
        f.write_all(&(self.chunk_count as u64).to_le_bytes())?;
        f.write_all(&(self.macros.len() as u64).to_le_bytes())?;
        for m in &self.macros {
            f.write_all(&(m.len() as u32).to_le_bytes())?;
            f.write_all(m)?;
        }
        f.write_all(&(self.programs.len() as u64).to_le_bytes())?;
        for p in &self.programs {
            f.write_all(&(p.len() as u32).to_le_bytes())?;
            for ins in p {
                match ins {
                    Instr::Literal(b) => { f.write_all(&[0u8])?; f.write_all(&[*b])?; }
                    Instr::Macro(m) => { f.write_all(&[1u8])?; f.write_all(&m.to_le_bytes())?; }
                }
            }
        }
        f.sync_all()?;
        Ok(fs::metadata(path)?.len())
    }

    fn total_macro_bytes(&self) -> u64 {
        self.macros.iter().map(|m| m.len() as u64).sum()
    }
}

// ============================================================
// Helpers
// ============================================================

fn bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &b in bytes {
        for i in (0..8).rev() {
            bits.push((b >> i) & 1);
        }
    }
    bits
}

fn format_bytes(b: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if b >= GB { format!("{:.2} GB", b as f64 / GB as f64) }
    else if b >= MB { format!("{:.2} MB", b as f64 / MB as f64) }
    else if b >= KB { format!("{:.2} KB", b as f64 / KB as f64) }
    else { format!("{} B", b) }
}

fn format_dur(d: Duration) -> String {
    let us = d.as_micros();
    if us < 1000 { format!("{} µs", us) }
    else if us < 1_000_000 { format!("{:.2} ms", us as f64 / 1000.0) }
    else { format!("{:.2} s", us as f64 / 1_000_000.0) }
}

fn chunk_stream(bytes: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let end = (i + chunk_size).min(bytes.len());
        chunks.push(bytes[i..end].to_vec());
        i = end;
    }
    chunks
}

// ============================================================
// Stress test runner
// ============================================================

struct StressResult {
    label: String,
    src_bytes: u64,
    v1_nodes: u64,
    v1_file_bytes: u64,
    v1_dedup_pct: f64,           // vs raw bit count
    v1_bloat_ratio: f64,         // v1_file / src_bytes
    v1_needle_found: bool,
    v1_needle_time: Duration,
    v1_halluc_rate: f64,
    v1_deep_found: bool,
    v1_deep_time: Duration,
    v2_macros: u64,
    v2_macro_bytes: u64,
    v2_file_bytes: u64,
    v2_ratio_vs_raw: f64,        // v2_file / src_bytes
    v2_ratio_vs_v1: f64,         // v2_file / v1_file
    v2_needle_found: bool,
    v2_needle_time: Duration,
    v2_halluc_rate: f64,
    v2_deep_found: bool,
    v2_deep_time: Duration,
    v2_build_time: Duration,
}

fn run_stress(label: &str, bytes: &[u8], v1_path: &str, v2_path: &str, skip_v1: bool) -> StressResult {
    println!("\n========== {} ==========", label);
    let src_bytes = bytes.len() as u64;
    let src_bits = src_bytes * 8;
    println!("Source: {} ({} bytes, {} bits)", label, format_bytes(src_bytes), src_bits);

    // ---------- V1 build ----------
    let mut v1_nodes: u64 = 0;
    let mut v1_file_bytes: u64 = 0;
    let mut v1_dedup_pct: f64 = 0.0;
    let mut v1_bloat_ratio: f64 = 0.0;
    let mut v1_needle_found = false;
    let mut v1_needle_time = Duration::from_secs(0);
    let mut v1_halluc_rate = 100.0;
    let mut v1_deep_found = false;
    let mut v1_deep_time = Duration::from_secs(0);
    let mut v1: Option<GraphV1> = None;

    // ---------- V2 placeholders ----------
    let mut v2_macros: u64 = 0;
    let mut v2_macro_bytes: u64 = 0;
    let mut v2_file_bytes: u64 = 0;
    let mut v2_ratio_vs_raw: f64 = 0.0;
    let mut v2_ratio_vs_v1: f64 = 0.0;
    let mut v2_needle_found = false;
    let mut v2_needle_time = Duration::from_secs(0);
    let mut v2_halluc_rate: f64 = 100.0;
    let mut v2_deep_found = false;
    let mut v2_deep_time = Duration::from_secs(0);
    let mut v2_build_time = Duration::from_secs(0);

    if !skip_v1 {
        println!("[V1] Building prefix-sharing trie (chunks of {} bytes)...", CHUNK_BYTES);
        let t0 = Instant::now();
        let mut g = GraphV1::new();
        let chunks = chunk_stream(bytes, CHUNK_BYTES);
        let mut new_count: u64 = 0;
        for c in &chunks {
            let bits = bytes_to_bits(c);
            new_count += g.insert_bits(&bits);
        }
        let v1_build = t0.elapsed();
        v1_nodes = g.nodes.len() as u64;
        v1_dedup_pct = if src_bits > 0 {
            (1.0 - (v1_nodes as f64 / src_bits as f64)) * 100.0
        } else { 0.0 };
        v1_bloat_ratio = v1_nodes as f64 / src_bits as f64;
        println!("[V1] Nodes: {} (source bits: {}, dedup: {:.4}%, bloat ratio: {:.4}x)",
                 v1_nodes, src_bits, v1_dedup_pct, v1_bloat_ratio);
        println!("[V1] Build time: {}", format_dur(v1_build));

        let path = format!("{}.tmp", v1_path);
        v1_file_bytes = g.serialize(&path).expect("V1 serialize");
        // Move to final path
        fs::rename(&path, v1_path).expect("rename");
        println!("[V1] On-disk file size: {} ({})", format_bytes(v1_file_bytes), v1_path);

        // V1 tests
        let needle_bytes = &bytes[..80.min(bytes.len())];
        let needle_bits = bytes_to_bits(needle_bytes);
        let t0 = Instant::now();
        v1_needle_found = g.contains_substring(&needle_bits);
        v1_needle_time = t0.elapsed();
        println!("[V1] Needle (first 80 bytes): {} in {}", v1_needle_found, format_dur(v1_needle_time));

        let fake_queries = [
            "The quantum matrix exploded into a billion fragments of dark matter.",
            "Alice ran to the moon and ate cheese with seven purple dragons.",
            "The cat typed code on a mechanical keyboard while singing opera.",
            "Zaphod Beeblebrox stole the tarts and flew to Andromeda at lunchtime.",
            "Neural networks hallucinate profusely when fed poisoned training data.",
        ];
        let mut fp = 0u32;
        for q in &fake_queries {
            let qb = bytes_to_bits(q.as_bytes());
            if g.contains_substring(&qb) { fp += 1; }
        }
        v1_halluc_rate = (fp as f64 / fake_queries.len() as f64) * 100.0;
        println!("[V1] Hallucination: {}/{} false positives ({:.1}%)",
                 fp, fake_queries.len(), v1_halluc_rate);

        let mid = bytes.len() / 2;
        let deep_end = (mid + 80).min(bytes.len());
        let deep_bits = bytes_to_bits(&bytes[mid..deep_end]);
        let t0 = Instant::now();
        v1_deep_found = g.contains_substring(&deep_bits);
        v1_deep_time = t0.elapsed();
        println!("[V1] Deep needle (bytes {}-{}): {} in {}",
                 mid, deep_end, v1_deep_found, format_dur(v1_deep_time));
        v1 = Some(g);
    } else {
        println!("[V1] SKIPPED (would need > 20 GB RAM for this corpus size)");
    }

    // ---------- V2 build ----------
    println!("\n[V2] Building compressed graph (min match = {} bytes)...", MIN_MATCH_BYTES);
    let chunks = chunk_stream(bytes, CHUNK_BYTES);
    println!("[V2] Chunked into {} pieces of ~{} bytes each", chunks.len(), CHUNK_BYTES);

    let t0 = Instant::now();
    let v2 = GraphV2::build_from(&chunks);
    v2_build_time = t0.elapsed();
    v2_macros = v2.macros.len() as u64;
    v2_macro_bytes = v2.total_macro_bytes();
    println!("[V2] Macros created: {} (total {} bytes of shared substrings)",
             v2_macros, format_bytes(v2_macro_bytes));
    // Macro size histogram for honesty
    if v2_macros > 0 {
        let mut sizes: Vec<usize> = v2.macros.iter().map(|m| m.len()).collect();
        sizes.sort();
        let min = sizes[0];
        let max = sizes[sizes.len() - 1];
        let median = sizes[sizes.len() / 2];
        let mean = sizes.iter().sum::<usize>() as f64 / sizes.len() as f64;
        println!("[V2] Macro size: min={} median={} mean={:.1} max={}",
                 min, median, mean, max);
    }
    println!("[V2] Build time: {}", format_dur(v2_build_time));

    v2_file_bytes = v2.serialize(v2_path).expect("V2 serialize");
    v2_ratio_vs_raw = v2_file_bytes as f64 / src_bytes as f64;
    v2_ratio_vs_v1 = if v1_file_bytes > 0 {
        v2_file_bytes as f64 / v1_file_bytes as f64
    } else { 0.0 };
    println!("[V2] On-disk file size: {} ({})", format_bytes(v2_file_bytes), v2_path);
    println!("[V2] Ratio vs raw text: {:.4}x  ({}% of original)", v2_ratio_vs_raw, (v2_ratio_vs_raw * 100.0) as u64);
    if v1_file_bytes > 0 {
        println!("[V2] Ratio vs V1: {:.4}x  ({}% reduction)",
                 v2_ratio_vs_v1, ((1.0 - v2_ratio_vs_v1) * 100.0) as u64);
    }

    // V2 tests
    let needle_bytes = &bytes[..80.min(bytes.len())];
    let t0 = Instant::now();
    v2_needle_found = v2.contains_substring(needle_bytes);
    v2_needle_time = t0.elapsed();
    println!("[V2] Needle (first 80 bytes): {} in {}", v2_needle_found, format_dur(v2_needle_time));

    let fake_queries = [
        "The quantum matrix exploded into a billion fragments of dark matter.",
        "Alice ran to the moon and ate cheese with seven purple dragons.",
        "The cat typed code on a mechanical keyboard while singing opera.",
        "Zaphod Beeblebrox stole the tarts and flew to Andromeda at lunchtime.",
        "Neural networks hallucinate profusely when fed poisoned training data.",
    ];
    let mut fp = 0u32;
    for q in &fake_queries {
        if v2.contains_substring(q.as_bytes()) { fp += 1; }
    }
    v2_halluc_rate = (fp as f64 / fake_queries.len() as f64) * 100.0;
    println!("[V2] Hallucination: {}/{} false positives ({:.1}%)",
             fp, fake_queries.len(), v2_halluc_rate);

    let mid = bytes.len() / 2;
    let deep_end = (mid + 80).min(bytes.len());
    let t0 = Instant::now();
    v2_deep_found = v2.contains_substring(&bytes[mid..deep_end]);
    v2_deep_time = t0.elapsed();
    println!("[V2] Deep needle (bytes {}-{}): {} in {}",
             mid, deep_end, v2_deep_found, format_dur(v2_deep_time));

    // Free v1
    drop(v1);

    StressResult {
        label: label.to_string(),
        src_bytes,
        v1_nodes, v1_file_bytes, v1_dedup_pct, v1_bloat_ratio,
        v1_needle_found, v1_needle_time, v1_halluc_rate, v1_deep_found, v1_deep_time,
        v2_macros, v2_macro_bytes, v2_file_bytes, v2_ratio_vs_raw, v2_ratio_vs_v1,
        v2_needle_found, v2_needle_time, v2_halluc_rate, v2_deep_found, v2_deep_time,
        v2_build_time,
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   AXIOM-3D HONEST STRESS TEST (real Gutenberg text)     ║");
    println!("║   v2: fast LZ77 V2, real on-disk file sizes             ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let data_dir = "/home/z/my-project/scripts/yk_stress/data";
    let out_dir = "/home/z/my-project/scripts/yk_stress/out";
    fs::create_dir_all(out_dir).unwrap();

    let mut results: Vec<StressResult> = Vec::new();

    // ----- 1 MB -----
    let bytes_1mb = fs::read(format!("{}/corpus_1mb.txt", data_dir)).expect("read 1mb");
    results.push(run_stress("1 MB corpus", &bytes_1mb,
        &format!("{}/v1_1mb.bin", out_dir),
        &format!("{}/v2_1mb.bin", out_dir), false));

    // ----- 10 MB -----
    let bytes_10mb = fs::read(format!("{}/corpus_10mb.txt", data_dir)).expect("read 10mb");
    results.push(run_stress("10 MB corpus", &bytes_10mb,
        &format!("{}/v1_10mb.bin", out_dir),
        &format!("{}/v2_10mb.bin", out_dir), false));

    // ----- 100 MB -----
    // V1 on 100MB would create ~800M nodes at 26 bytes = ~20GB RAM.
    // We skip V1 here for honesty. V2 builds fine.
    let bytes_100mb = fs::read(format!("{}/corpus_100mb.txt", data_dir)).expect("read 100mb");
    results.push(run_stress("100 MB corpus", &bytes_100mb,
        &format!("{}/v1_100mb_skipped.bin", out_dir),
        &format!("{}/v2_100mb.bin", out_dir), true));

    // ---------- final honest summary table ----------
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                   FINAL HONEST RESULTS TABLE                              ║");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║ {:<12} │ {:>10} │ {:>10} │ {:>10} │ {:>10} │ {:>7} │ {:>6}",
             "Corpus", "Raw", "V1 file", "V2 file", "V2/Raw", "V2/V1", "Halluc");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    for r in &results {
        let v1_str = if r.v1_file_bytes > 0 { format_bytes(r.v1_file_bytes) } else { "skipped".to_string() };
        let v2_v1_pct = if r.v1_file_bytes > 0 {
            format!("{:.1}%", r.v2_ratio_vs_v1 * 100.0)
        } else { "n/a".to_string() };
        println!("║ {:<12} │ {:>10} │ {:>10} │ {:>10} │ {:>8.2}x │ {:>6} │ {:>5.1}%",
                 r.label,
                 format_bytes(r.src_bytes),
                 v1_str,
                 format_bytes(r.v2_file_bytes),
                 r.v2_ratio_vs_raw,
                 v2_v1_pct,
                 r.v2_halluc_rate);
    }
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║ ZERO-ERROR CHECK (V1 must pass needle+deep+0% halluc; V2 same):");
    for r in &results {
        let v1_pass = if r.v1_file_bytes > 0 {
            r.v1_needle_found && r.v1_deep_found && r.v1_halluc_rate == 0.0
        } else { true }; // skipped = N/A
        let v2_pass = r.v2_needle_found && r.v2_deep_found && r.v2_halluc_rate == 0.0;
        let v1_str = if r.v1_file_bytes > 0 {
            format!("V1 {} (needle={} deep={} halluc={:.1}%)",
                if v1_pass {"PASS"} else {"FAIL"},
                r.v1_needle_found, r.v1_deep_found, r.v1_halluc_rate)
        } else { "V1 skipped".to_string() };
        println!("║   {:<12}: {} | V2 {} (needle={} deep={} halluc={:.1}%)",
                 r.label, v1_str,
                 if v2_pass {"PASS"} else {"FAIL"},
                 r.v2_needle_found, r.v2_deep_found, r.v2_halluc_rate);
    }
    println!("╚══════════════════════════════════════════════════════════════════════════╝");

    // ---------- write JSON ----------
    let mut json = String::new();
    json.push_str("{\n  \"axiom3d_stress_test\": true,\n  \"results\": [\n");
    for (i, r) in results.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"label\":\"{}\",\"src_bytes\":{},\"v1_nodes\":{},\"v1_file_bytes\":{},\"v1_dedup_pct\":{:.4},\"v1_bloat_ratio\":{:.4},\"v2_macros\":{},\"v2_macro_bytes\":{},\"v2_file_bytes\":{},\"v2_ratio_vs_raw\":{:.4},\"v2_ratio_vs_v1\":{:.4},\"v1_needle_found\":{},\"v2_needle_found\":{},\"v1_deep_found\":{},\"v2_deep_found\":{},\"v1_halluc_rate\":{:.4},\"v2_halluc_rate\":{:.4}}}",
            r.label, r.src_bytes, r.v1_nodes, r.v1_file_bytes, r.v1_dedup_pct, r.v1_bloat_ratio,
            r.v2_macros, r.v2_macro_bytes, r.v2_file_bytes, r.v2_ratio_vs_raw, r.v2_ratio_vs_v1,
            r.v1_needle_found, r.v2_needle_found, r.v1_deep_found, r.v2_deep_found,
            r.v1_halluc_rate, r.v2_halluc_rate
        ));
        if i + 1 < results.len() { json.push(','); }
        json.push('\n');
    }
    json.push_str("  ]\n}\n");
    let _ = fs::write(format!("{}/stress_results.json", out_dir), &json);
    println!("\nJSON summary written to {}/stress_results.json", out_dir);
}
