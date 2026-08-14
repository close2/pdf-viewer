# 500 — The key a dictionary lookup allocates

**Finding.** `Dictionary::get` was `self.0.get(&Name::new(key.as_bytes().to_vec()))` — a `Vec`
allocated and copied into, the `Arc<[u8]>` it is copied into again, and both freed, on **every
dictionary lookup in the program**. One cold search of ISO 32000-2 makes **3 278 302** calls to it.
`impl Borrow<[u8]> for Name` lets the map be probed with the caller's own bytes: **−1.92% of a
cold sweep**, `malloc` down 19.1% and `free` down 15.7%, with the readback of all 1023 pages
byte-identical. It costs no memory and four lines. The launch path gains 0.86% for free.

**Date.** 2026-08-14.
**ADR.** [0335](../adr/0335-the-key-a-dictionary-lookup-allocates.md).
**Touched.** `crates/pdf-syntax/src/object.rs` (the `Borrow` impl, `Dictionary::get`,
`Dictionary::remove`, a new `tests` module with two tests), `doc/conformance/ledger.toml` (§7.3.5's
row: the standing check, and a misquotation of the clause corrected), `doc/performance.md` (the
search section's closing paragraph), `doc/habits.md` (the pool-pinning rule below),
`doc/todo/47-search-performance.md` (**deleted** — the last item is taken and the argument is in
five ADRs), `doc/todo/README.md` (47's line), `doc/todo/10-bounds-that-cap-size.md`,
`crates/viewer-core/tests/headless.rs`, `crates/viewer-core/examples/find_cost.rs` and
`crates/pdf-model/examples/parallel_sweep.rs` (four live references to the deleted file, repointed
at the ADRs that hold the argument — a deleted todo whose citations survive it is a dangling link
in the code), `doc/adr/0335-*` (new), this file.

## The numbers, and how they were taken

Ten parallel rounds on the machine, so no wall clock at all: two binaries from one tree under
callgrind, ADR 0330's exact invocation so that the two rounds measure the same thing.

```sh
valgrind --tool=callgrind --callgrind-out-file=<arm>.out \
    <arm>/find_cost doc/ISO_32000-2_sponsored_EC3.pdf zzzqqqxyzzy 0 split 100000
```

| one cold sweep, ISO 32000-2, 1023 pages | instructions | |
|---|---|---|
| whole run | 37 642 044 068 → 36 920 639 974 | **−1.92%** |
| `Dictionary::get`, inclusive | 1 529 890 537 (4.06%) → 243 241 | the item |
| `interpret_with` / `Pages::get`, inclusive | 34 785 695 797 → 34 512 393 072 / 2 114 147 099 → 1 674 108 309 | −0.79% / −20.8% |
| `malloc` / `free`, self | 970 008 670 → 784 946 458 / 1 407 620 170 → 1 187 110 535 | −19.1% / −15.7% |

The before arm reproduces `doc/todo/47`'s figure for the item to 0.05% (1 529 118 804 recorded,
1 529 890 537 measured) and ADR 0330's whole-run figure to 0.09%, which is what says the arms are
comparable. Only 0.72 G of the 1.53 G comes off the run: the rest is the map's own search, which
stopped being a function — with no allocation left in the body `Dictionary::get` inlines, and the
comparisons reappear in `get_key_of` (+314 M), `resource_entry` (+45 M) and `get_key` (+15 M).

Two serial paths where lookups are the work: `callgrind_interpret` on page 101,
1 374 953 559 → 1 361 198 350 (**−1.00%**); `open_cost` on ISO 32000-2's launch path,
241 989 744 → 239 910 560 (**−0.86%**).

**Output identity was checked as bytes rather than as a count.** Both arms' `readback` example over
all 1023 pages, concatenated: 2 730 201 bytes, `sha256 ed074b1c…`, `cmp` silent. `find_cost`'s own
split line says 2 658 697 on both, and the corpus gate run on both arms differs in nothing but its
wall clock — 974 documents, 0 unopenable, 8 locked, 2 encrypted, 6 pageless, 62 incomplete, the
same three silence counts, the four lists identical as sorted sets.

## The gates

`cargo fmt --all --check` silent; `cargo clippy --workspace --all-targets` silent of lints (the
`viewer-qt` `cxx-qt` gcc warnings and the `proc-macro-error2` future-incompat note are `doc/todo/02`'s
known non-lints); `cargo nextest run --workspace` **1802 passed, 11 skipped** — the two new tests are
the whole of the change from the base; doctests all `ok`; `cargo test -p conformance` **5 passed**,
which is what says the two new rustdoc blockquotes are the standard's own words; the corpus gate
**974 documents, 62 incomplete**, identical to the before arm as above; `text_extraction`
**2 passed** — the pdf.js half **99.3% (24014/24193 words), 22 below 90%**, and the PDFBox half
skipping because `doc/corpora/pdfbox` is not checked out in this worktree, which the run says out
loud.

## The trap this round walked into, and it is a new one

**`open_one` on two large corpus documents first read +0.154% and +0.010% — a regression on a
change that cannot cost anything.** The diff named it in one line: `crossbeam_deque::Stealer::steal`
+1.24 M and +5.11 M, `WorkerThread::wait_until_cold` +0.42 M and +1.70 M. Callgrind counts every
thread, so a work-stealing pool's *spin* is in the total, and on a page whose real delta is a few
thousand instructions the spin is three orders of magnitude larger. With `RAYON_NUM_THREADS=1` both
documents read −0.003%, which is the truth: those pages are image work and a dictionary lookup is
0.002% of them.

`doc/performance.md` has said *quote the clock for a parallel change and the counter for a serial
one* since session 162. This is its converse and it was not written down anywhere: **pin the pool
before counting a serial change in a program that has one.** It is in `doc/habits.md` now.

## What the next round should know

- **The `Borrow` contract was checked before it was relied on, and the check is the interesting
  part.** A `BTreeMap` probed through a `Borrow` whose ordering disagrees with its keys' does not
  fail loudly — it silently stops finding entries that are there. `Name`'s five relevant traits are
  all *derived*, a derive on a one-field tuple struct delegates to the field and `Arc<[u8]>`
  delegates to `[u8]`, and no hand-written impl of any of them exists in the tree. That is a
  stronger statement than reading the code once, and `borrows_exactly_as_it_compares` keeps it
  true against a future hand-written impl.
- **§7.3.5's ledger note quoted a sentence the standard does not contain.** It read "PDF name
  objects are considered distinct objects if, after all escaping is expanded, the resulting
  sequences of bytes are not an exact binary match" — most of the words of the clause's actual
  sentence, in a different construction, inside quotation marks. Corrected against `doc/md/`. This
  is exactly the shape `doc/todo/02` §4's ledger-quotation sweep exists to find, and it was sitting
  in the row of the clause this round was working in.
- **The corpus gate's per-document wall-clock assertion has no tolerance and this machine can trip
  it.** Two runs failed on `22060_A1_01_Plans.pdf` at 35.2 s and 80.1 s against a 30 s budget while
  the load average was 119 to 136, and the same binary passes with the whole corpus at 24.1 s when it
  is not. `KNOWN_SLOW` is `[&str; 0]`. A round that sees this should check the *instruction count* of
  the named document before believing it — page one of that one is −0.003% here — and a round with
  the machine to itself might ask whether an assertion of that shape belongs in a gate at all.
- **`doc/todo/02` §5 was not run.** The release binaries a person runs are built from `main` and
  this round's target directory is the worktree's; whoever merges owns that section.
