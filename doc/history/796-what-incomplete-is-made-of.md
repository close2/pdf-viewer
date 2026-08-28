# 796 — What `incomplete` is made of

Date: 2026-08-28. Branch `round-796`, from `main` at `babb3f40`.
ADR: [0730](../adr/0730-what-incomplete-is-made-of.md).
Touched: `crates/pdf-model/tests/corpus.rs`, `crates/pdf-font/src/composite.rs`,
`crates/pdf-font/src/loading.rs`, `doc/conformance/ledger.toml` (§9.7.3, §9.7.5.2, §9.10.2),
`doc/todo/README.md`, and two new files — `doc/adr/0730` and
`doc/todo/18-a-flush-is-not-a-truncation.md`.

## The finding, in one sentence

The corpus gate's `incomplete` population is **many mechanisms dominated by one class** — the
great majority of it is the file's own defect rather than work this reader owes — and the largest
single mechanism in it is one sentence of §9.7.5.2 that the file breaks; the composition is now
printed by the gate instead of being kept by hand in a comment that had drifted to three
different figures for one quantity.

## What was read

Every document on the list, with its reports, against `Unsupported`'s own doc comments and the
clauses they cite. The exact figures are the gate's and are not written here; what the reading
found is:

- The mechanism carrying more than a quarter of the population is `/Encoding /Identity-H` over a
  descendant with no embedded program — the combination §9.7.5.2 says "shall not be used" — with
  no `/ToUnicode` either, so §9.10.2's three methods are exhausted. ADR 0433 had read part of this
  population off the ink sweep by hand and reached the same conclusion; nothing said so at the
  point a reader meets the message.
- Next largest are malformed content streams, then embedded font programs that will not parse, a
  `/MediaBox` no ancestor states, names no resource dictionary defines, and an image dictionary
  its own codestream contradicts. All of them the file's.
- A small remainder is closed by a reading or a decision (a subset with no glyph for any code its
  own document shows, a bound this program set, a transparency model this tree departs from where
  §11.4 and §11.6.2 let it), and a very small remainder is work owed.

Every document on the list was also checked individually where its report was ambiguous.
`issue4575.pdf` writes `/Width /Height` and no `/Height` at all; `sci-notation.pdf` writes `ETBT`
with no white space between two keywords, which §7.2.3 makes one token; `bug1953099.pdf` writes a
bare `-` inside a `TJ` array. Each is the file, and each report is right.

## What changed in the program

`pdf-font`'s composite-font path raised one message for four different facts about a file, and
only the last of them is a gap in this reader — §9.7.5.2's forbidden combination, a descendant
with no `/CIDSystemInfo` Table 115 makes required, an `Identity` ordering §9.7.3 makes the glyph
order of a program nobody supplied, and a character collection this binary carries no table for.
`composite::collection_gap` says which, and the refusal becomes `FontError::NoSubstitute` rather
than `UnsupportedEncoding`, because the encoding is read perfectly well and what fails is reaching
a substitute through it. No pixel moves; the same documents report the same number of times.

## What changed in the instrument

`whose_defect` and `print_the_composition` in the corpus gate. Every report is placed under a
mechanism and one of three classes — the file's, neither one's, this reader's — the classes are
printed with a carrying count and a partition, and **a report the table cannot place fails the
gate**. There is no `other` row, deliberately: an `other` row is what let the old table drift.

## Two things worth keeping

**A class is a claim about a clause.** One row was written from the shape of the message and was
wrong: a JBIG2 refusal reads like a codec gap, and §7.4.7 says the JBIG2 file header "shall not be
present" in an embedded stream — the corpus's one witness is `jbig2_file_header.pdf`, named for
carrying the header it may not carry, so a segment the decoder calls unknown or reserved is one
ISO/IEC 14492 does not define. Writing this table is a reading, not a tidying-up.

**Trap 13 earned its keep, twice.** Calibrating the unplaced assertion by breaking a marker on
purpose left the gate *green* — the first draft of the font arm ended in a broad
`"cannot be substituted"` row that swallowed the §9.7.5.2 case above it, so eighteen documents
would have been silently reclassified by any rewording, and the classification's own output looked
entirely correct throughout. A marker table with a catch-all in it is not a table.

**The comment that drifted had promised not to.** It carried the sentence "recomputed every
session because a number nothing recomputes is a number that drifts" beside the table, twice, in
two different corrections. A promise is not an instrument, and this is the clearest instance of
ADR 0281's rule this tree has.

## What was found and not taken

`doc/todo/18` — three of the incomplete documents are reported damaged and are not. Their form
`XObject` is a `zlib` stream flushed with `Z_SYNC_FLUSH` and never finished: every byte the
encoder produced is decoded, `comments.pdf`'s object 667 giving all 648 bytes of its content, and
only RFC 1950's trailer is absent. The file has the measurement (three of the pdf.js corpus's 974,
four of a 12 000-file sample of the crawl), the decidable test, and the one thing that blocks it —
zlib framing then wants an `ADLER32` this tree does not compute.

Also seen and not this round's: `sci-notation.pdf`'s `1e2` is read as 100 by the lexer's
`parse::<f64>()` fallback, and §7.3.3 admits no exponent in either numeric form. It costs no
report and no mark on that page, but it is a departure nothing states, and the sentence beside it
in `read_number` is about exactly this failure shape.

## The gitlink guard covered four of six, and this round tripped it

`git add -A crates doc` in this worktree staged **type changes** over `doc/pdf.js` and
`doc/arlington-pdf-model`: `tools/worktree.sh` symlinks six submodules and set `--skip-worktree`
on only the four under `doc/corpora/`, so `worktree.sh list` printed `4/4 skip-worktree` — the
guard reporting itself *on* while two gitlinks were bare. `cargo test -p conformance`'s
`every_declared_submodule_is_still_tracked_as_one` failed, which is exactly what that gate is for,
and `git reset` put both back at mode 160000 byte-identically before anything was committed.

The script fix belongs to **session 794**, which found the same thing on its own branch, so this
round reverted its duplicate and kept the diffs disjoint. What is recorded here is the shape,
because it is trap 11 in a shell script: a guard whose *report* is computed from a hard-coded
population tells you it is on when it is covering two thirds of what it names.

## Gates

Run whole, in the worktree, with sibling rounds 794, 795 and 797 live on the same machine — which
`doc/todo/02` §2 says inflates every line that spawns a reference renderer, so the wall clocks
below are of a loaded machine and the verdicts are not.

| line | what it printed |
|---|---|
| `cargo fmt --all --check` | clean |
| `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` | clean |
| `cargo nextest run --workspace` | 2743 tests run, 2743 passed, 18 skipped |
| `cargo test --workspace --doc` | all green |
| `cargo check --manifest-path fuzz/Cargo.toml --bins` | clean |
| corpus | 974 documents: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 67 incomplete, 0 slow — and the composition, printed for the first time |
| oracle | 983 agrees, 61 contradicted, 836 ambiguous, 3 our geometry, 2 reference geometry, 42 not comparable, 18 no render |
| text extraction | 508 of 974 judged, 10971/11163 matched words in bounds (98.28%), 487 of 508 documents fully in bounds |
| selection census | green |
| accessibility census | 102 853 elements, 7413 clickable, 57 116 a caret moves through |
| dates, XMP, JPEG 2000 | green |
| quorra corpus | 957 pages: 932 agree, 22 differ, 3 refused, 17 not comparable; median 2.04× the CPU backend |
| fixed documents | 41 checked, 0 absent, 41 rows |
| `cargo test -p conformance` | green |

`not comparable` at 42 is the load speaking rather than the tree: `doc/todo/02` §2 records the
same line at 38 under a parallel sweep and 13 alone. The ratchet held, and no verdict moved.

The corpus gate, the two font tests and the conformance gate were re-run after the last edit,
which is the rule about a number being current only for the round that ran the gate last.

## Sweeps

§4's sweeps run over `crates/`, `tools/`, `fuzz/`, `doc/adr/` and the ledger, before against
pristine `main` at `babb3f40` and after: every delta is a line number moving under an edit, plus
`overtaken`'s decision-record count rising by one for ADR 0730. No finding appeared and none
disappeared. One pre-existing hit moved with the comment it is in and was left where it is —
`corpus.rs`'s narrative still names §8.9.7 as unimplemented, which it has not been for a long
time, and rewriting that paragraph is a bigger edit than this round should make beside a
neighbour's.

Three deltas are content rather than line numbers, and each is accounted:

- **`entries` finds fewer.** §9.7.4.1, §9.7.5.3 and §9.7.6.1 each lose an entry their own code did
  not name, because `composite::collection_gap` now reads `/CIDSystemInfo`, `/Registry`,
  `/Ordering` and `/Encoding` by name where nothing in those rows' files had.
- **`callers` and `counts` find more**, by one file and by the sentences these documents added.
- **`pointers` finds two fewer, and that one was a *finding about this round*.** The first draft
  numbered the new todo `17`, and `17` is a number a deleted item used to have: eleven documents
  cite `doc/todo/17` for a shading's wash, a linearised unknown filter and three other things, and
  creating a file there silently made every one of those dangling pointers resolve — to the wrong
  item. The sweep is what said so, by *losing* two standing findings between the before and the
  after. The file is `18`, which nothing cites. **A free number in `doc/todo/` is not the same
  thing as an unused one**, and the check is a grep over `doc/` and `crates/` before the file is
  written.
