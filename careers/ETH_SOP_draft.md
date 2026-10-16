# ETH Zurich Student Summer Research Fellowship — Statement of Purpose

**Applicant:** Yashas K
**ORCID:** 0009-0005-8928-5341
**Program:** ETH Student Summer Research Fellowship 2027
**Application window:** Opens early November 2026
**URL:** https://inf.ethz.ch/studies/summer-research-fellowship.html

---

## Statement of Purpose (draft, ~600 words)

I am applying to the ETH Student Summer Research Fellowship to advance my independent research on succinct data structures for exact-match retrieval. Specifically, I want to upgrade my AXIOM prototype from a prefix trie with $O(N \cdot L)$ substring search to a generalized suffix tree with $O(L)$ substring search, and to evaluate the resulting system on real retrieval-augmented generation (RAG) workloads.

### Background

I am a 2nd-year Computer Science engineering student in Bangalore, India. Outside coursework, I have spent the past year building AXIOM, a multi-layer binary substring graph for zero-error retrieval. The project is public at \url{https://github.com/Yashas-K-Gangatkar/axiom-3d} and is documented in an 8-page IEEE conference format paper with 14 references.

AXIOM has three layers, each stress-tested on real Project Gutenberg text at 1, 10, and 100 MB:

- **V1** is a prefix-sharing binary trie. It is the baseline, but it bloats 212× on disk because each bit of input requires a 26-byte node.
- **V2** is a Lempel-Ziv-77 macro graph that shares repeated substrings across documents. It honestly compresses to 0.86× on 100 MB of repetitive text, but is 38× worse than `xz -9` because it lacks entropy coding.
- **V3.1** is a LOUDS succinct trie (Jacobson 1989) that reduces V1's storage by 94% by encoding each node in 4.5 bits instead of 26 bytes.

The headline result is a 0% hallucination rate across all corpus sizes, verified against five adversarial fake queries per size. The exact-match property is structural: there is no probability anywhere in the retrieval path.

### What I want to do at ETH

AXIOM's main algorithmic weakness is that substring search is $O(N \cdot L)$ rather than $O(L)$. The natural fix is to replace V1's prefix trie with a generalized suffix tree using Ukkonen's online construction. This is the work I want to do at ETH.

The Systems for ML group at ETH (Ce Zhang, Laurent Vanbever) and the Institute of Machine Learning (Thomas Hofmann, Andreas Krause) have the right combination of low-level systems expertise and ML application context to mentor this work. A suffix-tree upgrade to AXIOM would:

1. Reduce deep-needle search from $O(N \cdot L)$ to $O(L)$, making AXIOM competitive with classical CS data structures on substring search.
2. Enable end-to-end evaluation of AXIOM as a verification gate behind FAISS-class ANN indexes on a real RAG benchmark (e.g., Natural Questions).
3. Position AXIOM for submission to IEEE Big Data 2027 or a NeurIPS workshop on efficient ML.

### Why ETH specifically

I am applying to ETH because:

- The ETH Department of Computer Science has a strong record on data structures and information retrieval (Ukkonen's suffix-tree work is canonical in this area).
- The Student Summer Research Fellowship explicitly accepts undergraduates who demonstrate independent research potential, which my AXIOM repository and paper provide.
- ETH has direct connections to the Swiss AI ecosystem (Google Zurich, Disney Streaming, Microsoft Zurich), which aligns with my longer-term goal of pursuing a research career in Switzerland.
- The two-month summer format (July--August 2027) fits my academic calendar.

### Honest limitations of my current work

I want to be explicit about what AXIOM is not: it is not a better compressor than `gzip` or `xz`; it is not a faster search engine than FAISS; it does not replace Transformer-based LLMs. The contribution is a complementary infrastructure layer that provides exact substring retrieval with shared-substring dedup---a property structurally absent from ANN-based systems. ETH's mentorship would help me close the algorithmic gap to classical suffix-tree methods and quantify where AXIOM genuinely adds value in a RAG pipeline.

### What I will bring

- A working, public, reproducible Rust codebase with real measurements
- An IEEE-format paper draft ready for revision
- Self-direction: I built AXIOM without a mentor, so I know how to formulate and execute a research agenda
- Honest documentation: my repo explicitly lists failure modes, which I would continue at ETH

I would be honored to spend summer 2027 at ETH advancing this work.

---

## Application checklist

- [ ] Submit application at https://inf.ethz.ch/studies/summer-research-fellowship/how-to-apply.html (window opens early November 2026)
- [ ] Attach CV (use the Disney resume as a base, tailor for academic context)
- [ ] Attach transcript
- [ ] Attach this Statement of Purpose
- [ ] Attach AXIOM paper PDF
- [ ] List 1-2 ETH faculty as preferred mentors (research their recent papers first)
- [ ] Ask 1 academic reference for a letter (any professor who can speak to your independent work)

## ETH faculty to consider as potential mentors

- **Ce Zhang** — Systems for ML, CodaLab, deep learning systems. AXIOM's systems angle fits.
- **Andreas Krause** — LAZY Lab, adaptive data structures, Bayesian optimization.
- **Thomas Hofmann** — Institute of Machine Learning, former Google Research Zurich.
- **Ryan Cotterell** — NLP, computational linguistics. AXIOM's text retrieval angle.
- **Laurent Vanbever** — Networking systems, programmable data planes.

Read 2-3 recent papers from each before naming them in your application.
