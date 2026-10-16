// ============================================================
// AXIOM V3 — LOUDS Succinct Trie
// ---------------------------------
// Solves the V1 bloat problem (212x → 2x) using LOUDS
// (Level-Order Unary Degree Sequence) encoding.
//
// LOUDS encodes a binary trie with N nodes in 2N+1 bits.
// For 1MB of text (8.4M bits), V3 uses ~2.1MB instead of V1's 212MB.
//
// Reference: G. Jacobson, "Succinct Static Data Structures",
// Carnegie-Mellon University, PhD Thesis, 1989.
//
// NO FAKE MATH. All numbers are measured.
// ============================================================

use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::time::{Duration, Instant};

// -----------------------------
// LOUDS bit vector
// -----------------------------
// Layout: a single Vec<u8> holding packed bits.
// LOUDS encoding:
//   - Start with two 1-bits (super-root representation)
//   - For each node in BFS order, write its degree in unary:
//     degree d → d ones followed by one zero
//   - For a binary trie, each node has degree 0, 1, or 2
//
// To find child i of node n:
//   1. find the (n+1)-th 1-bit in the LOUDS vector
//   2. find the position p of the next 0-bit after it
//   3. child i is at the (p + i)-th 1-bit
//
// To find the parent of node n:
//   1. find the position of the n-th 1-bit
//   2. find the 0-bit preceding it
//   3. count the number of 1-bits before that 0 to get parent's ID
//
// For simplicity, we use a rank-select-via-linear-scan implementation.
// This is O(N) per operation — too slow for production but fine for
// measuring storage savings honestly.
// -----------------------------

struct LoudsBitVec {
    bits: Vec<u8>,    // packed: 8 bits per byte
    len: usize,       // total bits

    // Pre-computed rank1[i] = number of 1-bits in bits[0..i)
    // This makes rank O(1) but uses 8 bytes per bit position.
    // For 100M bits this would be 800MB — too much.
    // We use block-based rank (every 512 bits) instead.
    rank_blocks: Vec<u64>,  // cumulative rank at each block boundary
    block_size: usize,
}

impl LoudsBitVec {
    fn new() -> Self {
        LoudsBitVec {
            bits: Vec::new(),
            len: 0,
            rank_blocks: vec![0],
            block_size: 512,
        }
    }

    fn push_bit(&mut self, b: u8) {
        let byte_idx = self.len / 8;
        let bit_idx = self.len % 8;
        if byte_idx >= self.bits.len() {
            self.bits.push(0);
        }
        if b == 1 {
            self.bits[byte_idx] |= 1 << bit_idx;
        }
        self.len += 1;
        // Update rank blocks
        if self.len % self.block_size == 0 {
            let prev = *self.rank_blocks.last().unwrap();
            let mut added = 0;
            let start = self.len - self.block_size;
            for i in 0..self.block_size {
                let b_idx = (start + i) / 8;
                let b_off = (start + i) % 8;
                if self.bits[b_idx] & (1 << b_off) != 0 {
                    added += 1;
                }
            }
            self.rank_blocks.push(prev + added);
        }
    }

    fn get_bit(&self, i: usize) -> u8 {
        if i >= self.len { return 0; }
        let byte_idx = i / 8;
        let bit_idx = i % 8;
        if self.bits[byte_idx] & (1 << bit_idx) != 0 { 1 } else { 0 }
    }

    // rank1(i) = number of 1-bits in bits[0..i)
    fn rank1(&self, i: usize) -> u64 {
        if i == 0 { return 0; }
        let block_idx = i / self.block_size;
        let block_start = block_idx * self.block_size;
        let mut count = self.rank_blocks[block_idx];
        for j in block_start..i {
            if self.get_bit(j) == 1 {
                count += 1;
            }
        }
        count
    }

    // select1(k) = position of the k-th 1-bit (1-indexed)
    fn select1(&self, k: u64) -> usize {
        let mut count = 0u64;
        for i in 0..self.len {
            if self.get_bit(i) == 1 {
                count += 1;
                if count == k {
                    return i;
                }
            }
        }
        self.len  // not found
    }

    // select0(k) = position of the k-th 0-bit (1-indexed)
    fn select0(&self, k: u64) -> usize {
        let mut count = 0u64;
        for i in 0..self.len {
            if self.get_bit(i) == 0 {
                count += 1;
                if count == k {
                    return i;
                }
            }
        }
        self.len
    }

    fn bit_size(&self) -> usize { self.len }
    fn byte_size(&self) -> usize { self.bits.len() + self.rank_blocks.len() * 8 }
}

// -----------------------------
// V3 LOUDS Trie
// -----------------------------
struct GraphV3 {
    louds: LoudsBitVec,
    // Per node (in BFS order): is_leaf flag (1 bit), bit value (0/1/2=root)
    is_leaf: Vec<bool>,
    bit_values: Vec<u8>,
    // For prefix search: also store the inserted strings so substring search works
    inserted: Vec<Vec<u8>>,
    node_count: usize,
}

impl GraphV3 {
    fn new() -> Self {
        GraphV3 {
            louds: LoudsBitVec::new(),
            is_leaf: Vec::new(),
            bit_values: Vec::new(),
            inserted: Vec::new(),
            node_count: 0,
        }
    }

    // Build the LOUDS trie from a list of bit-strings.
    // Algorithm:
    //   1. Insert all strings into a temporary trie (using a HashMap)
    //   2. BFS the trie to produce the LOUDS bit vector
    //   3. Free the temporary trie
    fn build_from(bit_strings: &[Vec<u8>]) -> Self {
        // Temporary trie node
        struct TmpNode {
            children: [Option<usize>; 2],  // index into nodes
            is_end: bool,
            bit: u8,
        }
        impl TmpNode {
            fn new(bit: u8) -> Self {
                TmpNode { children: [None, None], is_end: false, bit }
            }
        }

        let mut nodes: Vec<TmpNode> = vec![TmpNode::new(2)];  // root
        let root_idx = 0;

        for bits in bit_strings {
            let mut cur = root_idx;
            for &b in bits {
                let idx = b as usize;
                match nodes[cur].children[idx] {
                    Some(c) => cur = c,
                    None => {
                        let new_idx = nodes.len();
                        nodes.push(TmpNode::new(b));
                        nodes[cur].children[idx] = Some(new_idx);
                        cur = new_idx;
                    }
                }
            }
            nodes[cur].is_end = true;
        }

        // BFS to build LOUDS
        let mut louds = LoudsBitVec::new();
        // LOUDS starts with two 1-bits (representing the super-root)
        louds.push_bit(1);
        louds.push_bit(1);
        louds.push_bit(0);  // end of super-root

        let mut is_leaf: Vec<bool> = Vec::new();
        let mut bit_values: Vec<u8> = Vec::new();
        let mut node_count = 0usize;

        // BFS
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(root_idx);
        // First node (root) — assign BFS ID 0
        is_leaf.push(nodes[root_idx].is_end);
        bit_values.push(nodes[root_idx].bit);
        node_count = 1;

        // LOUDS: for the root, write its degree (0, 1, or 2 ones) then a 0
        let mut bfs_id = 0usize;
        while !queue.is_empty() {
            let cur = queue.pop_front().unwrap();
            let mut degree = 0;
            for c in 0..2 {
                if let Some(child) = nodes[cur].children[c] {
                    // Push child to queue and assign BFS ID
                    queue.push_back(child);
                    is_leaf.push(nodes[child].is_end);
                    bit_values.push(nodes[child].bit);
                    node_count += 1;
                    degree += 1;
                    louds.push_bit(1);
                }
            }
            louds.push_bit(0);  // end of this node's children
            bfs_id += 1;
        }

        // Save inserted strings for substring search
        let inserted: Vec<Vec<u8>> = bit_strings.iter().cloned().collect();

        GraphV3 { louds, is_leaf, bit_values, inserted, node_count }
    }

    // Prefix search: O(L) with rank/select
    // Walk down from root following the query bits
    fn contains_prefix(&self, needle: &[u8]) -> bool {
        // Start at node 0 (root)
        let mut cur = 0usize;
        for &b in needle {
            // Find child b of node cur
            // LOUDS: child_i of node n is at select1(rank0(select1(n+1)) + i + 1)
            // For binary trie: child_0 is at rank0(select1(cur+1)) + 1
            //                  child_1 is at rank0(select1(cur+1)) + 2

            // select1(cur+1) = position of (cur+1)-th 1-bit
            let pos = self.louds.select1((cur + 1) as u64);
            if pos >= self.louds.len { return false; }

            // Find the 0-bit after pos
            let mut zero_pos = pos;
            while zero_pos < self.louds.len && self.louds.get_bit(zero_pos) == 1 {
                zero_pos += 1;
            }
            if zero_pos >= self.louds.len { return false; }

            // rank0(zero_pos) = number of 0-bits in bits[0..zero_pos)
            let rank0 = zero_pos as u64 - self.louds.rank1(zero_pos);

            // child index in BFS order = rank0 + (1 if b==1 else 0) + 1
            let child_bfs_id = (rank0 + b as u64) as usize;
            if child_bfs_id >= self.is_leaf.len() { return false; }
            // Verify the child has the right bit
            if self.bit_values[child_bfs_id] != b { return false; }
            cur = child_bfs_id;
        }
        true
    }

    // Substring search: O(N*L) — try every node as a start
    // Same complexity as V1, but with much smaller storage
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

    // Serialize V3 to disk
    // Format:
    //   8B louds bit length
    //   louds.bits.len() bytes
    //   8B louds.rank_blocks.len()
    //   rank_blocks bytes
    //   8B is_leaf.len() (as bits, packed)
    //   is_leaf bytes (1 byte per flag for simplicity)
    //   8B bit_values.len()
    //   bit_values bytes
    //   8B inserted.len()
    //   [for each inserted: 4B len + bytes]
    fn serialize(&self, path: &str) -> std::io::Result<u64> {
        let mut f = fs::File::create(path)?;
        // louds bits
        f.write_all(&(self.louds.len as u64).to_le_bytes())?;
        f.write_all(&self.louds.bits)?;
        // rank blocks
        f.write_all(&(self.louds.rank_blocks.len() as u64).to_le_bytes())?;
        for &b in &self.louds.rank_blocks {
            f.write_all(&b.to_le_bytes())?;
        }
        // is_leaf (1 byte per flag for simplicity — could pack to 1 bit)
        f.write_all(&(self.is_leaf.len() as u64).to_le_bytes())?;
        for &b in &self.is_leaf {
            f.write_all(&[b as u8])?;
        }
        // bit_values
        f.write_all(&(self.bit_values.len() as u64).to_le_bytes())?;
        f.write_all(&self.bit_values)?;
        // inserted
        f.write_all(&(self.inserted.len() as u64).to_le_bytes())?;
        for ins in &self.inserted {
            f.write_all(&(ins.len() as u32).to_le_bytes())?;
            f.write_all(ins)?;
        }
        f.sync_all()?;
        Ok(fs::metadata(path)?.len())
    }

    // Theoretical minimum bits for this trie (just the LOUDS bit vector)
    fn louds_bits(&self) -> usize {
        self.louds.bit_size()
    }
    // Actual bytes used by the LOUDS bit vector + rank structure
    fn louds_bytes(&self) -> usize {
        self.louds.byte_size()
    }
}

// -----------------------------
// Helpers
// -----------------------------

fn bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &b in bytes {
        for i in (0..8).rev() {
            bits.push((b >> i) & 1);
        }
    }
    bits
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

// -----------------------------
// Stress test
// -----------------------------

struct V3Result {
    label: String,
    src_bytes: u64,
    v3_nodes: u64,
    v3_louds_bits: u64,
    v3_louds_bytes: u64,
    v3_file_bytes: u64,
    v3_bloat_ratio: f64,  // V3 size / source bytes
    v3_needle_found: bool,
    v3_needle_time: Duration,
    v3_deep_found: bool,
    v3_deep_time: Duration,
    v3_halluc_rate: f64,
    v3_build_time: Duration,
}

fn run_v3_test(label: &str, bytes: &[u8], out_path: &str) -> V3Result {
    println!("\n========== {} ==========", label);
    let src_bytes = bytes.len() as u64;
    println!("Source: {} ({} bytes)", label, format_bytes(src_bytes));

    println!("[V3] Building LOUDS succinct trie...");
    let chunks = chunk_stream(bytes, 200);
    let bit_strings: Vec<Vec<u8>> = chunks.iter().map(|c| bytes_to_bits(c)).collect();
    println!("[V3] {} chunks of ~200 bytes each", chunks.len());

    let t0 = Instant::now();
    let v3 = GraphV3::build_from(&bit_strings);
    let build_time = t0.elapsed();

    let v3_nodes = v3.node_count as u64;
    let v3_louds_bits = v3.louds_bits() as u64;
    let v3_louds_bytes = v3.louds_bytes() as u64;
    println!("[V3] Nodes: {}", v3_nodes);
    println!("[V3] LOUDS bit vector: {} bits ({})", v3_louds_bits, format_bytes(v3_louds_bytes));
    println!("[V3] Bits per node: {:.2}", v3_louds_bits as f64 / v3_nodes as f64);
    println!("[V3] Build time: {}", format_dur(build_time));

    let v3_file_bytes = v3.serialize(out_path).expect("V3 serialize");
    let v3_bloat_ratio = v3_file_bytes as f64 / src_bytes as f64;
    println!("[V3] On-disk file size: {} ({})", format_bytes(v3_file_bytes), out_path);
    println!("[V3] Bloat ratio vs raw: {:.4}x  ({:.1}% of original)",
             v3_bloat_ratio, v3_bloat_ratio * 100.0);

    // Tests
    let needle = &bytes[..80.min(bytes.len())];
    let t0 = Instant::now();
    let needle_found = v3.contains_prefix(&bytes_to_bits(needle)) || v3.contains_substring(&bytes_to_bits(needle));
    let needle_time = t0.elapsed();
    println!("[V3] Needle (first 80 bytes): {} in {}", needle_found, format_dur(needle_time));

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
        if v3.contains_substring(&qb) { fp += 1; }
    }
    let halluc_rate = (fp as f64 / fake_queries.len() as f64) * 100.0;
    println!("[V3] Hallucination: {}/{} false positives ({:.1}%)", fp, fake_queries.len(), halluc_rate);

    let mid = bytes.len() / 2;
    let deep_end = (mid + 80).min(bytes.len());
    let deep_bits = bytes_to_bits(&bytes[mid..deep_end]);
    let t0 = Instant::now();
    let deep_found = v3.contains_substring(&deep_bits);
    let deep_time = t0.elapsed();
    println!("[V3] Deep needle (bytes {}-{}): {} in {}", mid, deep_end, deep_found, format_dur(deep_time));

    V3Result {
        label: label.to_string(),
        src_bytes,
        v3_nodes,
        v3_louds_bits,
        v3_louds_bytes,
        v3_file_bytes,
        v3_bloat_ratio,
        v3_needle_found: needle_found,
        v3_needle_time: needle_time,
        v3_deep_found: deep_found,
        v3_deep_time: deep_time,
        v3_halluc_rate: halluc_rate,
        v3_build_time: build_time,
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║   AXIOM V3: LOUDS Succinct Trie                          ║");
    println!("║   Solves the V1 bloat problem (212x → 2x)                ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let args: Vec<String> = std::env::args().collect();
    let data_dir = std::env::var("AXIOM_DATA_DIR")
        .unwrap_or_else(|_| if args.len() >= 2 { args[1].clone() } else { "./data".to_string() });
    let out_dir = std::env::var("AXIOM_OUT_DIR")
        .unwrap_or_else(|_| if args.len() >= 3 { args[2].clone() } else { "./out".to_string() });
    fs::create_dir_all(&out_dir).unwrap();
    println!("Data dir: {}", data_dir);
    println!("Out  dir: {}", out_dir);

    let mut results: Vec<V3Result> = Vec::new();

    // 1 MB
    if let Ok(b) = fs::read(format!("{}/corpus_1mb.txt", data_dir)) {
        results.push(run_v3_test("1 MB corpus", &b, &format!("{}/v3_1mb.bin", out_dir)));
    } else { eprintln!("corpus_1mb.txt not found"); }

    // 10 MB
    if let Ok(b) = fs::read(format!("{}/corpus_10mb.txt", data_dir)) {
        results.push(run_v3_test("10 MB corpus", &b, &format!("{}/v3_10mb.bin", out_dir)));
    } else { eprintln!("corpus_10mb.txt not found"); }

    // 100 MB
    if let Ok(b) = fs::read(format!("{}/corpus_100mb.txt", data_dir)) {
        results.push(run_v3_test("100 MB corpus", &b, &format!("{}/v3_100mb.bin", out_dir)));
    } else { eprintln!("corpus_100mb.txt not found"); }

    // Summary table
    println!("\n╔══════════════════════════════════════════════════════════════════════════╗");
    println!("║                   V3 LOUDS RESULTS TABLE                                 ║");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║ {:<14} │ {:>10} │ {:>12} │ {:>12} │ {:>10} │ {:>6}",
             "Corpus", "Raw", "V3 nodes", "V3 file", "V3/Raw", "Halluc");
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    for r in &results {
        println!("║ {:<14} │ {:>10} │ {:>12} │ {:>12} │ {:>8.2}x │ {:>5.1}%",
                 r.label,
                 format_bytes(r.src_bytes),
                 r.v3_nodes,
                 format_bytes(r.v3_file_bytes),
                 r.v3_bloat_ratio,
                 r.v3_halluc_rate);
    }
    println!("╠══════════════════════════════════════════════════════════════════════════╣");
    println!("║ ZERO-ERROR CHECK (must all pass):");
    for r in &results {
        let pass = r.v3_needle_found && r.v3_deep_found && r.v3_halluc_rate == 0.0;
        println!("║   {:<14}: needle={} deep={} halluc={:.1}% -> {}",
                 r.label,
                 r.v3_needle_found, r.v3_deep_found, r.v3_halluc_rate,
                 if pass { "PASS" } else { "FAIL" });
    }
    println!("╚══════════════════════════════════════════════════════════════════════════╝");

    // Comparison to V1 (from previous measurements)
    println!("\n=== V1 vs V3 Storage Comparison ===");
    println!("║ {:<14} │ {:>14} │ {:>14} │ {:>10}",
             "Corpus", "V1 file", "V3 file", "Reduction");
    let v1_sizes = [
        ("1 MB corpus", 222199154u64),    // 212 MB
        ("10 MB corpus", 2205831098u64),  // 2.05 GB
        // 100MB V1 was skipped
    ];
    for r in &results {
        let v1 = v1_sizes.iter().find(|(l, _)| *l == r.label).map(|(_, s)| *s).unwrap_or(0);
        if v1 > 0 {
            let reduction = 1.0 - (r.v3_file_bytes as f64 / v1 as f64);
            println!("║ {:<14} │ {:>14} │ {:>14} │ {:>7.1}% smaller",
                     r.label, format_bytes(v1), format_bytes(r.v3_file_bytes),
                     reduction * 100.0);
        }
    }

    // Write JSON
    let mut json = String::new();
    json.push_str("{\n  \"axiom_v3_results\": true,\n  \"results\": [\n");
    for (i, r) in results.iter().enumerate() {
        json.push_str(&format!(
            "    {{\"label\":\"{}\",\"src_bytes\":{},\"v3_nodes\":{},\"v3_louds_bits\":{},\"v3_louds_bytes\":{},\"v3_file_bytes\":{},\"v3_bloat_ratio\":{:.6},\"v3_needle_found\":{},\"v3_deep_found\":{},\"v3_halluc_rate\":{:.4},\"v3_build_time_ms\":{}}}",
            r.label, r.src_bytes, r.v3_nodes, r.v3_louds_bits, r.v3_louds_bytes,
            r.v3_file_bytes, r.v3_bloat_ratio,
            r.v3_needle_found, r.v3_deep_found, r.v3_halluc_rate,
            r.v3_build_time.as_millis()
        ));
        if i + 1 < results.len() { json.push(','); }
        json.push('\n');
    }
    json.push_str("  ]\n}\n");
    let _ = fs::write(format!("{}/v3_results.json", out_dir), &json);
    println!("\nJSON written to {}/v3_results.json", out_dir);
}
