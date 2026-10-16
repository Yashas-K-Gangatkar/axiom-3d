# Rewrite AXIOM in Python — Make It Genuinely Yours

If learning Rust feels like too much, rewrite AXIOM in a language you already know. Python is the natural choice — it's slower, but the algorithm is the same, and you'll understand every line because you wrote it.

## Why Python is a valid choice

- **You actually own the code.** No AI assistance needed for the implementation if you know basic Python.
- **The algorithm is the same.** A trie is a trie in any language. LZ77 is LZ77 in any language. LOUDS is LOUDS in any language.
- **Python is acceptable for prototypes.** Research code is often Python. Disney Streaming uses C++/Rust/Java for production, but for a research prototype demonstrating architectural ideas, Python is fine.
- **Slower execution is honest.** You can document: "Python implementation; expected ~50× slower than Rust. The architectural conclusions are language-independent."

## What the Python version should contain

### V1 — Binary trie (easiest, start here)

```python
class Node:
    def __init__(self, bit: int):
        self.bit = bit
        self.child0 = None
        self.child1 = None
        self.is_end = False

class Trie:
    def __init__(self):
        self.root = Node(2)  # 2 = root sentinel
    
    def insert_bits(self, bits: list[int]) -> int:
        """Insert a list of 0/1 values. Return new nodes created."""
        new_nodes = 0
        cur = self.root
        for b in bits:
            child = cur.child0 if b == 0 else cur.child1
            if child is not None:
                cur = child
            else:
                n = Node(b)
                new_nodes += 1
                if b == 0:
                    cur.child0 = n
                else:
                    cur.child1 = n
                cur = n
        cur.is_end = True
        return new_nodes
    
    def contains_prefix(self, needle: list[int]) -> bool:
        cur = self.root
        for b in needle:
            child = cur.child0 if b == 0 else cur.child1
            if child is None:
                return False
            cur = child
        return True
```

That's it. ~30 lines. You can write this yourself. Test it on small inputs first:

```python
def bytes_to_bits(data: bytes) -> list[int]:
    bits = []
    for b in data:
        for i in range(7, -1, -1):
            bits.append((b >> i) & 1)
    return bits

t = Trie()
t.insert_bits(bytes_to_bits(b"hello"))
print(t.contains_prefix(bytes_to_bits(b"hel")))  # True
print(t.contains_prefix(bytes_to_bits(b"xyz")))  # False
```

### V2 — LZ77 macro graph (medium difficulty)

The Python version uses a dictionary for the hash chain instead of raw arrays. Slightly slower, much easier to read.

```python
def build_v2(chunks: list[bytes], min_match: int = 8, max_match: int = 64) -> dict:
    """Build a V2-style LZ77 macro graph.
    Returns dict with 'macros' (list of bytes) and 'programs' (list of lists).
    """
    macros = []
    macro_index = {}  # bytes -> macro_id
    programs = []
    seen = {}  # bytes -> (chunk_idx, position)
    
    for chunk in chunks:
        program = []
        i = 0
        while i < len(chunk):
            best_len = 0
            best_macro = None
            max_possible = min(len(chunk) - i, max_match)
            
            for L in range(min(max_possible, 64), min_match - 1, -1):
                candidate = bytes(chunk[i:i+L])
                if candidate in macro_index:
                    best_len = L
                    best_macro = macro_index[candidate]
                    break
            
            if best_len >= min_match and best_macro is not None:
                program.append(('macro', best_macro))
                # Register substrings of this match
                for s in range(best_len):
                    for L in range(min_match, min(best_len - s, 256) + 1):
                        sub = bytes(chunk[i+s:i+s+L])
                        if sub not in seen:
                            seen[sub] = (len(programs), i + s)
                i += best_len
            else:
                program.append(('literal', chunk[i]))
                # Register new substrings ending here
                start = max(0, i - 255)
                for s in range(start, i + 1):
                    sub = bytes(chunk[s:i+1])
                    if len(sub) >= min_match and sub not in seen:
                        seen[sub] = (len(programs), s)
                i += 1
        
        programs.append(program)
    
    return {'macros': macros, 'programs': programs}
```

This is roughly 50 lines. The hash chain is replaced with a `dict`, which is slower but conceptually identical.

### V3.1 — LOUDS (advanced, optional)

LOUDS in Python is tricky because Python doesn't have bit-level memory layout. You'd use Python's `int` as a bit vector:

```python
class LoudsBitVec:
    def __init__(self):
        self.bits = 0  # Python int, arbitrary precision
        self.len = 0
    
    def push_bit(self, b: int):
        if b:
            self.bits |= (1 << self.len)
        self.len += 1
    
    def get_bit(self, i: int) -> int:
        if i >= self.len:
            return 0
        return (self.bits >> i) & 1
    
    def rank1(self, i: int) -> int:
        return bin(self.bits & ((1 << i) - 1)).count('1')
    
    def select1(self, k: int) -> int:
        count = 0
        for i in range(self.len):
            if self.get_bit(i) == 1:
                count += 1
                if count == k:
                    return i
        return self.len
```

Note: Python's `select1` is O(N) here. That's honest — Python is slower. Document it.

## Step-by-step plan

### Week 1: V1 in Python
- [ ] Write `Node` and `Trie` classes (use the code above as a starting point, but type it yourself)
- [ ] Write `bytes_to_bits` and `bits_to_bytes` helpers
- [ ] Write a test: insert "hello" and "help", verify both findable, verify "xyz" not found
- [ ] Run on the 1MB corpus. Measure: how many nodes? How long to build? How long to search?

### Week 2: V2 in Python
- [ ] Write the LZ77 macro graph builder (above)
- [ ] Write a function to materialize a chunk (replace macro refs with bytes)
- [ ] Write substring search: materialize each chunk, naive search
- [ ] Run on 1MB corpus. Compare file size to V1.

### Week 3: V3.1 in Python (optional, harder)
- [ ] Write the `LoudsBitVec` class
- [ ] Write the LOUDS trie builder (BFS over a regular trie)
- [ ] Write the prefix search using rank/select
- [ ] Run on 1MB corpus. Verify 94% size reduction vs V1.

### Week 4: Tests and benchmarks
- [ ] Write 5+ unit tests (use pytest)
- [ ] Run all 3 versions on 1MB, 10MB corpora
- [ ] Write a `results.json` with all measurements
- [ ] Update the paper with Python numbers (honest: "Python implementation; ~50× slower than Rust")

### Week 5: Update README and resume
- [ ] Add `python/` directory to the repo with your implementation
- [ ] Update README to say "AXIOM is implemented in both Rust (research prototype) and Python (reference implementation)"
- [ ] Update resume to claim Python fluency (which you actually have, because you wrote the code)

## Why this is the honest path

After 5 weeks of Python work:
- You own 100% of the Python implementation
- You can defend every line in an interview
- You can claim Python fluency honestly
- The Rust version remains as "the high-performance implementation, developed with AI assistance, which I'm currently learning to read and modify"

This is honest. This is defensible. This is what you should do.

## What to put on your resume after this

> "Designed and implemented AXIOM, a binary substring graph for zero-error retrieval, in Python (reference implementation) and Rust (high-performance implementation). The Python implementation is entirely my own work; the Rust implementation was developed with AI assistance and I am actively studying Rust to fully own it."

This is true. This is defensible. This will get you interviews.

## The alternative: don't apply yet

If 5 weeks of Python work is too much, **don't apply to Disney or ETH yet.** Wait until you've done the work. The Swiss AI job market is small and your reputation matters. A failed interview where you can't defend your own code is worse than no interview.

Apply in January 2027 (for summer 2027 internships) after completing the Python rewrite in November–December 2026. That gives you a real, defensible project.
