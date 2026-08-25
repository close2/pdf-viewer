# 752 — The link nobody priced, and the tag every byte was asked

General-improvement round, chosen rather than assigned. Subject: **`doc/todo/43` §1 — what the fat
link is for**, open since ADR 0222 and restated as owed in two later rounds. The briefing's rule
was that the highest-yield general round finds a number this project wrote down and has not re-run;
this one found three copies of the same number, all decayed, and an answer that reversed the
question's own expectation. It then found a defect underneath the measurement and shipped it.

ADRs **0666** (the link) and **0667** (the predictor). 0668 unused.

## Why this subject

Three siblings were on the confinement interrupt's policy, the oracle's *we are alone* ranking and
the errata-ranked ledger rows, so those were out. `doc/todo/43` §1 stood out because it names
itself a **measurement task, not a change**, states the candidate to test, and had gone three
hundred rounds untouched — and because the A/B turned out to need **no edit at all**:
`[profile.gates]` is declared `inherits = "release"` with `lto = "thin"` and `codegen-units = 16`
and nothing else, so the two arms were already in `Cargo.toml`. Two more profiles (`thin1`,
`fat16`) separated the settings and were removed before the commit.

## What the measurement said

Callgrind rather than a clock — `doc/todo/43`'s own warning about its denominator, and the machine
carried three parallel rounds at load 1.7 to 35 throughout. **Every arm reproduced to the
instruction across three passes**, so there is no spread to read a difference against.

| against `release` (`fat`, 1) | `callgrind_open` | `callgrind_interpret` | `callgrind_rasterise` |
|---|---|---|---|
| `gates` (`thin`, 16) | **+12.30%** | +1.65% | +4.07% |
| `thin`, 1 | +4.06% | +1.84% | +0.67% |
| `fat`, 16 | +10.72% | −0.01% | −0.00% |

**Both settings are load-bearing and on different paths** — cross-crate inlining is what the
interpreter and rasteriser gain, the single code generation unit is what §7.5's cross-reference
parse gains, and no cheaper combination reproduces the pair. The fat profile's binaries are also
about a quarter smaller. **Decision: `[profile.release]` is unchanged**, and the question is closed
rather than open.

## The three decayed numbers

`cargo build --timings`, §5's six binaries, after touching one file in `pdf-model`:

| | wall | `viewer-ui`'s unit | CPU, all units |
|---|---|---|---|
| `release` | 94.5 s | **93.59 s** | 241.0 s |
| `gates` | 50.4 s | 49.48 s | 178.2 s |

- `doc/todo/43` §1's "66.4 s of 76.9" → **93.59 s**
- `doc/todo/43`'s table row "§5's three release binaries, 79.3 s" → **94.5 s**, and §5 names six
  binaries and a shared library now
- `Cargo.toml`'s own comment above `[profile.release]`, "78 s" → **93.59 s**

All three corrected. **`viewer-ui`'s link *is* §5's critical path** — it starts 0.8 s in and ends
0.1 s before the build does, with `viewer-gtk`, `viewer-qt`, `viewer-confined` and `pdf-retrieve`
all finishing 45 s underneath it — so the prize was never the sum of the links. And it had been
priced against a cadence ADR 0428 removed: §5 runs every fifth round now, so ~44 s became ~9 s a
round.

## What was underneath it — ADR 0667

The per-function attribution showed the function whose *inlining* moved between arms:
`filter::apply_predictor`, standing alone at 238.6 M of 857.1 M under a thin link. Two readings —
**§7.4.4.4's predictor is roughly a quarter of `Document::open`**, which no document in this tree
said, and **a loop whose cost swings by double digits with the optimiser's mood is written to
depend on the optimiser.**

It was asking, once per byte, which filter the *row* had declared, and fetching a left and an
upper-left neighbour that types 0 and 2 never read — where `/Predictor 12`, type 2 on every row, is
what essentially every cross-reference stream uses, six bytes a row and 101 318 rows on this file.
The tag now selects a loop.

| | before | after | |
|---|---|---|---|
| `callgrind_open`, ten opens | 763,278,781 | 678,200,421 | **−11.15%** |
| per `Document::open` | 76.33 M | **67.82 M** | |
| `callgrind_interpret` | 1,294,054,067 | 1,285,546,294 | −0.66% |
| `callgrind_rasterise` | 5,448,437,924 | 5,450,359,321 | +0.035%, code layout |

**The attribution is exact**: `decode_with_parms_reported` falls 85,078,360, which is the entire
change in the total, and `xref::read_section` is identical to the instruction in both.

## What the change was checked with, and how the check was checked

A temporary differential example held the old implementation verbatim beside the new one over
200 000 generated cases — all predictor codes, all bit depths, lengths straddling row boundaries —
**0 disagreements**. Trap 13 says to run it against the defect first; three plants, each caught:
the empty-row case (2190), Paeth's arguments transposed (asymmetric, unlike `midpoint` — trap
746's lesson), and `Sub` starting a byte late.

**Two gaps the split made visible**, both now permanent tests: PNG filter type 4, **Paeth, had no
test at all**, and its new one is built so the three positions answer *up*, *up-left* and *left* in
turn, because a case that picked one neighbour throughout would also pass against a decoder that
implemented `Up`. And a **trailing type byte with no row** is accepted without its type being
examined — what the per-byte loop did, a statement about malformed input, and the thing a naive
hoist silently changes.

## The sequence, whole — `pdf-syntax` is in the "everything" row

Both workers built first. `fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 ·
doctests · fuzz check, exit 0 · `nextest` **2658 passed, 18 skipped** · corpus · oracle **exit 0,
all ratchets held, 142.6 s** · text extraction · both censuses · dates · XMP · JPEG 2000 ·
`render-quorra` · `fixed_documents` · conformance **192**. Sweeps `quotations` and `pointers`, both
exit 0. §5's binaries built and installed, and `pdf-retrieve` read the specification's 1023 pages
and page 101's text back through the shipped binary.

**The load is worth recording rather than hiding**: neighbours put the average between 1.7 and 35
across the round, and the oracle's 142.6 s reflects that rather than anything in this tree — §2's
own note puts a loaded oracle at 218 s against 57 quiet. It passed on every ratchet, which is what
the gate is. Everything the round *concluded* is an instruction count, which is why the load
changes none of it. `PDFREF_CACHE` was the shared warm cache throughout.

## What is left where this round was

- `doc/todo/43` §1 is closed; its remaining half-measure (skipping §5 when nothing reaches
  `viewer-ui`) is untouched and still unmeasured.
- ADR 0667 declines three things by argument: no hand-written SIMD (principle 3, in the crate that
  most wants `#![forbid(unsafe_code)]`), no change to the TIFF predictor, and no rewrite of `Sub`,
  `Average` and `Paeth` to shed their remaining bounds checks — the case that matters is free of
  them and no corpus document ranks the rest.
- The `+0.035%` on `callgrind_rasterise` is real and is code layout. It is recorded rather than
  explained away.
