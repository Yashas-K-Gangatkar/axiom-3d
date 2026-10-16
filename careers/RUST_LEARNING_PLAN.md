# Rust Learning Plan for AXIOM — 12 Weeks to Owning the Code

You need to be able to walk into an interview and answer questions about the AXIOM codebase. This plan takes you from "I don't know Rust" to "I can defend every line of V1 and explain V2/V3.1 at a high level" in 12 weeks, assuming 2 hours/day.

## Why 12 weeks

You're a 2nd-year engineering student with coursework. 2 hours/day × 7 days × 12 weeks = 168 hours. That's enough to be genuinely competent in Rust at a basic-intermediate level — not expert, but enough to defend a small codebase in an interview.

If you have less time, do the **Minimum Viable Path** (8 weeks, marked with ⭐ below).

## Resources (all free)

1. **The Rust Programming Language** (the "Book") — https://doc.rust-lang.org/book/
2. **Rustlings** (interactive exercises) — https://github.com/rust-lang/rustlings
3. **Rust by Example** — https://doc.rust-lang.org/rust-by-example/
4. **AXIOM source code** — your own repo

## Week 1–2: The basics ⭐

**Goal:** Write and run Rust programs. Understand variables, types, functions, control flow.

- [ ] Read Book chapters 1–3 (Getting Started, Programming a Guessing Game, Common Programming Concepts)
- [ ] Complete Rustlings exercises: `variables`, `functions`, `if`, `primitive_types`
- [ ] Install Rust on your Mac: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- [ ] Write 5 small programs: hello world, fibonacci, fizzbuzz, reverse a string, count words in a file

**Self-test:** Can you write a function that takes a `Vec<u8>` and returns the count of bytes equal to 0? If yes, move on.

## Week 3–4: Ownership and borrowing ⭐

**Goal:** Understand Rust's defining feature. This is the hardest part.

- [ ] Read Book chapters 4 (Ownership) and 10.3 (Lifetimes)
- [ ] Complete Rustlings: `move_semantics`, `structs`, `enums`
- [ ] Write a function that takes ownership of a `String` and returns its length
- [ ] Write a function that borrows a `&[u8]` slice and returns the max byte
- [ ] **Read AXIOM's `axiom_stress.rs` lines 1–100.** Identify every ownership transfer and borrow. You should be able to explain WHY each `&` or `&mut` is there.

**Self-test:** Can you explain to a friend why this doesn't compile?
```rust
let s = String::from("hello");
let t = s;
println!("{}", s);
```
If yes, move on.

## Week 5–6: Structs, enums, and collections ⭐

**Goal:** Build data structures in Rust.

- [ ] Read Book chapters 5 (Structs), 6 (Enums), 8 (Common Collections)
- [ ] Complete Rustlings: `collections`, `structs`, `enums`
- [ ] **Implement a binary trie node in Rust yourself.** Don't look at AXIOM. Define a `Node` struct with `bit: u8`, `child0: Option<u64>`, `child1: Option<u64>`, `is_end: bool`. Write `new()`, `insert()`, `search()` methods.
- [ ] Compare your implementation to AXIOM's V1. What's the same? What's different? Why?

**Self-test:** Can you implement a binary trie from scratch in Rust without looking at AXIOM? If yes, you own V1.

## Week 7–8: Traits, generics, error handling ⭐ (Minimum Viable Path ends here)

**Goal:** Write idiomatic Rust. Read the rest of AXIOM's source.

- [ ] Read Book chapters 7 (Packages), 9 (Error Handling), 10 (Generic Types and Traits)
- [ ] Complete Rustlings: `error_handling`, `generics`, `traits`
- [ ] **Read all of `axiom_stress.rs` (V1 + V2).** You should now understand:
  - How `GraphV1::insert_bits` works
  - How `GraphV2::build_from` works (the LZ77 hash chain)
  - Why `u64::MAX` is used as a sentinel
  - How serialization works

**Self-test:** Can you explain to a friend what `match child { c if c != u64::MAX => c, _ => { ... } }` does in the V1 insert function? If yes, you own V1 + V2.

## Week 9–10: LOUDS and V3.1

**Goal:** Understand the succinct data structure layer.

- [ ] Read the AXIOM paper Section III-C (V3.1 LOUDS)
- [ ] Read Jacobson's thesis summary (search "LOUDS succinct data structures Jacobson")
- [ ] **Read `axiom_v31.rs` line by line.** Focus on:
  - `LoudsBitVec::push_bit`, `get_bit`, `rank1`, `select1`
  - `GraphV31::build_from` (the BFS that produces the LOUDS bit vector)
  - `GraphV31::contains_prefix` (the rank/select navigation)
- [ ] Implement your own LOUDS bit vector in Rust. Just the bit vector, not the trie. Test it: push 1000 bits, verify rank1 and select1 work.

**Self-test:** Can you explain why LOUDS uses 2 bits per node instead of 26 bytes per node? Can you derive the rank/select formulas from the LOUDS definition?

## Week 11–12: Rewrite V1 from scratch

**Goal:** Prove you own the code by writing it yourself.

- [ ] Create a new file: `src/bin/axiom_my_v1.rs`
- [ ] Implement a binary trie in Rust from scratch. No AI assistance for the actual code (you can ask AI to explain concepts, but not to write the code).
- [ ] Your V1 should support:
  - `new()` — create an empty trie
  - `insert(bits: &[u8])` — insert a bit string
  - `contains_prefix(needle: &[u8]) -> bool` — prefix search
  - `contains_substring(needle: &[u8]) -> bool` — substring search
- [ ] Test it on the 1MB corpus. Compare your numbers to AXIOM's V1.
- [ ] If your numbers match (within ±5%), you genuinely own V1.

**Self-test:** Can you write V1 from scratch in under 2 hours, without any AI assistance, in front of an interviewer? If yes, you can defend the AXIOM repo.

## After Week 12: What to do in interviews

When asked about AXIOM in an interview, say this:

> "AXIOM is a research project I designed. The architecture — three layers, V1/V2/V3.1 — is mine. The initial implementation was developed with AI assistance, but I've since rewritten V1 myself in Rust to fully own the codebase. I can walk you through any part of it."

Then walk them through YOUR V1 implementation (from Week 12). Don't claim you wrote V2 or V3.1 unless you actually rewrite those too.

## What if you don't have 12 weeks?

If you have a job interview in 2 weeks, do this instead:

1. **Read the AXIOM paper** (1 day)
2. **Read `axiom_stress.rs` V1 only** (2 days) — ignore V2 and V3.1
3. **Be honest in the interview:** "I designed the system; the implementation was AI-assisted; I'm currently learning Rust to fully own it. I can walk you through the architecture and the design decisions."

Honesty + a clear learning plan is better than fake fluency. Interviewers can tell.

## The brutal truth

If you complete this 12-week plan, you will be employable as a junior Rust developer. If you don't, you won't be. There's no shortcut. The code in your repo is currently not yours — it becomes yours when you can write it yourself.

Start Week 1 today.
