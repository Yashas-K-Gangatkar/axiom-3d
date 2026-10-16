# Contributing to AXIOM

Thank you for your interest in contributing to AXIOM. This document describes how to contribute honestly and effectively.

## Code of Conduct

Be honest. Be kind. Be rigorous.

- **Honest**: Never fabricate benchmarks, inflate numbers, or hide failures. If a change makes things worse, say so in the commit message.
- **Kind**: Reviewers and contributors treat each other with respect. Personal attacks are not tolerated.
- **Rigorous**: Every claim must be backed by a measurement. Every measurement must be reproducible.

## How to contribute

### Reporting bugs

Open an issue at https://github.com/Yashas-K-Gangatkar/axiom-3d/issues with:
1. The exact command you ran
2. The exact output you got
3. What you expected
4. Your Rust version (`rustc --version`)
5. Your OS

If the bug affects a number in the paper, label the issue `paper-impact`.

### Suggesting improvements

Open an issue with the label `enhancement`. Describe:
1. What you want to add
2. Why it's an improvement (with measurements if possible)
3. Whether you're willing to implement it

### Submitting code

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/my-improvement`)
3. Make your changes
4. Add or update tests if applicable
5. Run `cargo test --release` to verify nothing breaks
6. Run the relevant benchmarks and include the new numbers in your PR description
7. Open a pull request

### What we accept

- Bug fixes (always welcome)
- Performance improvements (with measurements showing the speedup)
- Storage reductions (with measurements showing the byte savings)
- New test cases
- Documentation improvements
- Honest refactoring

### What we reject

- Changes that make the numbers look better without honest measurement
- "Optimizations" that skip work without verifying correctness
- Marketing language in the paper or README
- Comparisons to other systems that aren't apples-to-apples

## Development setup

```bash
git clone https://github.com/Yashas-K-Gangatkar/axiom-3d.git
cd axiom-3d
cargo build --release
./scripts/setup_and_run.sh  # build corpora
cargo run --release --bin axiom_stress
```

## Running tests

```bash
cargo test --release
```

Currently, tests are inline in each binary's main.rs. We're working on extracting them to a proper test suite.

## Style guide

- Rust code: follow `rustfmt` defaults
- LaTeX: 4-space indent, lines under 80 chars where possible
- Markdown: GitHub-flavored, no emojis
- Commit messages: imperative mood ("Add V3.1 LOUDS trie" not "Added V3.1 LOUDS trie")

## Acknowledgments

Contributors will be listed in the paper's acknowledgments section (if they want to be).
