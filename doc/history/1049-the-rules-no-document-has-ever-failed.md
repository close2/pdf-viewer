# 1049 — The rules no document has ever failed

Instruments slot, on `doc/todo/62` §7's other question: a row `examples/withdrawn.rs` finds no
document failing is a rule nobody has judged. Re-measured over the seven read corpora rather than
taken from ADR 1026 — the same eleven of 175 rows, and the list is in the run rather than here.

## What was built

`crates/pdf-archive/tests/unwitnessed.rs`, 25 tests: for ten of the eleven rows the smallest
document its clause says must fail and the smallest that must pass, plus three pinning a clause's
*condition* — a `Differences` array absent, a program with no vertical metrics, and the same
`DestOutputProfileRef` under a PDF/A rather than a PDF/X output intent. Calibrated as trap 13
asks: with the predicates replaced by `Check::Unchecked` in a scratch worktree all nine failing
fixtures went green and every pass mirror stayed green. Two builders are the reusable part —
`icc`, a profile from its header and tag table, and `sfnt`/`truetype`/`cmap`, a program the font
loader accepts; the first `cmap` mapped no code, so the loader refused the font instead of the
rule firing, which the comment above `cmap` now says.

Ten clause numbers, each cited and paraphrased above its own fixture: ISO 19005-4 sections 6.2.9,
6.2.3, 6.2.4.2, 6.2.10.5, 6.2.10.6 and 6.7.2.1, ISO 19005-2 sections 6.2.3, 6.2.4.2, 6.1.13 and
6.6.2.1. Two are about where the parts *differ*, which is what a pass mirror holds.

**No reach defect.** The corpus was searched for a witness of each row first: one file states
`DestOutputProfileRef` and it is not a PDF/X intent, the only undefined `BM` is in a graphics
state, and no file states `DeviceN`. The silence is the corpus's.

**ADR 1026 §5.1 was wrong about `implementation-limits/indirect-object-count`**, which it blamed on
the document `withdrawn.rs` skips by name. That document states 40 015 indirect objects and fails a
different row; this one has no witness at all, and its smallest failing document is 8 388 608
objects, each fetched by `Examination::objects` — too large for a test, so only the mirror is
pinned and the test says why. No row was re-statused; every predicate fired as its clause asks.

## Gates

`fmt -p pdf-archive --check` 0 · `clippy -p pdf-archive --all-targets` (`-D warnings`) 0 ·
`nextest -p pdf-archive --no-fail-fast` 100, 245/246 · `--doc` 0 · fuzz fmt 0, fuzz clippy 0 ·
`pdf-archive --test corpus` 0, `over` 0 on all six · `pdf-transform --test archive_corpus` 0 ·
`archive_census` unconsidered 0 on all six. Three red lines are siblings': `pdf-signature` will not
lint; `submission.rs` cites §17.13.4.1 and §17.6 and quotes uncited; and 1044's `cross_check.rs`
and `resource_fallbacks.rs` write "ADR 1055 §5", which `editions` reads as a clause citation.
