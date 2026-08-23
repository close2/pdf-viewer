# 680 — The two notes left, and the mechanism taken out of the file

The last two of ADR 0497's eight middle-bucket groups, against its sixth criterion. Parallel round,
worktree `r680`, branch `round-680`. **No pixel moves and no list changes**: what changed is two
group notes, three ledger rows, two documents and three paragraphs of trap 9. ADR 0510 has the
argument.

## What was owed and what it came to

675 took six of the eight and left `SUBSTITUTED_FONT` (8 pages) and `DEVICE_CMYK_CONVERSION` (5).
Both turned out to be its middle case — *the mechanism accounts for the bound, but only an ablation
could show that* — and both ablations came out at the strong end:

| group | outcome |
|---|---|
| `DEVICE_CMYK_CONVERSION` | the press owns **100% of every failing bound on all five pages** |
| `SUBSTITUTED_FONT` | the face owns every failing bound on **seven of eight**; the eighth at 82%, with its second mechanism named |

So no group in that bucket was an unearned exemption, which is the answer to the question 672 asked
and 675 half-answered.

## The two instruments

- **Colour**: a §7.5.6 incremental update giving each page a `/DefaultCMYK` (§8.6.5.6) naming a
  press. That reaches only our own code, so the counterfactual is exactly "if our source assumption
  had been theirs" and the comparison is against the references' renders of the *original* file.
- **Fonts**: `gs -sDEVICE=pdfwrite -dEmbedAllFonts=true -dSubsetFonts=false` with
  `<</NeverEmbed[]>> setdistillerparams`, which embeds the face the references already resolve, so
  §9.5 NOTE 5's mechanism cannot act for anybody. On seven of the eight documents all three
  references render the rewritten file **byte-identically** to the original, which is as clean as a
  control gets.

## Three things worth carrying

- **The differing fraction is a threshold count.** `JUST_NOTICEABLE` is 4 and alpha never differs
  between two opaque rasters, so a flat mark whose colour is off by (3, 3, 6) contributes its own
  area **÷ 4**, not `× 3 ÷ 4`. On `transparent.pdf` the bottle is 11.4175% of the page, blue alone
  crosses the threshold, and 11.50 ÷ 4 = 2.875 of the 3.316 points the gate prints against a bound
  of 1.38%. **Two levels of one channel decide that verdict.**
- **The gate's printed line is the worst-ratio member of the *agreeing consensus*, not of every
  reference.** `bug847420.pdf`'s four numbers are `mupdf`'s although `ghostscript` is further from
  us on all four. A before-and-after read against the wrong set compares two populations.
- **A profile predicts a renderer over the colours somebody sampled.** Trap 9's sixth bullet says
  our own evaluator on *either* SWOP-family profile predicts all three renderers to eight levels.
  On the same group's fifth page — one flat `0.82 0.7 0.54 0.67 k` ink — the **Artifex** profile
  that `mupdf` and `ghostscript` both read puts us eleven levels from all three, while `hayro`'s
  CGATS profile is within one. Not the intent (Artifex's `A2B0` and `A2B1` are the same 41 478
  bytes) but the black point: `icc.rs::detect_black` already recorded that a colorimetric
  construction and Little CMS's `B2A` round trip "agree everywhere except in the darkest few
  percent", and that sentence now has its number.

## And two figures and a comparison that never reproduced

`SUBSTITUTED_FONT`'s `bug847420.pdf` paragraph opened on a hand-built levels ranking at "8.65 of 255
from the nearest of four renderers that agree among themselves to 4.64, twice as far as any page on
the list that is not a link border". The **unit is real** — this is not ADR 0499's misread-unit
shape — and all three figures are wrong about their operand: 8.65 is our distance from `hayro`, the
*furthest* of the four and the one that does not vote; the nearest is `poppler` at 7.44; the four
references' six pairwise means run 1.38 to 3.48; and `issue15716.pdf` sits 13.96 from its nearest.
The note's own ink ladder reproduces to three decimals, so the rasters have not moved — these were
the wrong end of the range when they were written.

## Measured

Every §2 gate, whole (fifth round), all green and no count moved: `fmt`, `clippy --workspace
--all-targets` under `RUSTFLAGS="-D warnings"`, the fuzz check, `nextest` 2441 passed and 17
skipped, doctests, corpus, oracle **908 agrees / 65 contradicted / 786 ambiguous / 2 reference
geometry / 13 not comparable / 18 no render**, text extraction, both censuses, dates, XMP,
JPEG 2000, `render-quorra` **957 pages at glyph quantum 1/16 — 933 agree, 22 differ, 2 refused**,
`fixed_documents` 40 checked and 0 absent, conformance 182 + 5 + 1. The ledger's statuses did not
move. §5's binaries rebuilt and installed.

The oracle ran against the shared reference cache (`PDFREF_CACHE`), which is what kept a machine at
load 30 to 50 — four rounds building at once — from turning a budget into a verdict; its six counts
are identical before and after the edits and identical to 679's on `main`.

Sweeps: `--bin overtaken` clean on both rewritten notes; `--bin quoted` against this round's own
oracle log reads **151 figures, 88 confirmed, 50 contradicted** where the merged tree read 142 / 79 /
50 — nine figures added, all nine confirmed, and the contradicted count did not move.

## Changed

- `crates/pdf-model/tests/oracle.rs` — two notes, doc comments only.
- `doc/conformance/ledger.toml` — §10.3.2, §9.5 and §8.6.5.9, plus two stale counts retired in §9.5.
- `doc/traps/oracle-and-references.md` — trap 9: the threshold-count arithmetic, the consensus-set
  rule, and a scope correction to the sixth bullet.
- `doc/oracle-and-corpus.md` — the same three figures, which it repeated, and one stale count.
- `doc/todo/21-font-substitution.md` — §4 gains what the ablation prices, and loses a stale count.
- ADR 0510.

## Owed

- **Nothing links a group's note to *which* bound the gate fails its pages on.** All thirteen
  diagnoses here began by reading that off a log by hand, and a note can go on explaining a mean
  while the page fails on a differing fraction for as long as nobody looks. `--bin quoted` checks a
  figure a note quotes; it cannot ask for one that is missing.
- Unchanged from 675 and 679: a voting reference whose raster is constant still votes; the negatives
  queue; `freeculture.pdf` page 255; the owner's `git stash drop`.
