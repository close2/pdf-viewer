# 802 — The region a stroke covers is not only a path

Date: 2026-08-28. Branch `round-802`, from `main` at `46289075`. Parallel round, worktree `r802`.
ADR: [0735](../adr/0735-the-region-a-stroke-covers-is-not-only-a-path.md).
Touched: `crates/pdf-model/src/content/pattern.rs`, `.../path.rs`, `.../text.rs`, `.../image.rs`,
`crates/pdf-model/tests/tiling.rs`, `crates/pdf-model/tests/corpus.rs`,
`doc/conformance/ledger.toml` (§8.7.3), and two new files — `doc/adr/0735` and this one.

## The subject, and why the instruments chose it

The batch's general-improvement round, told to let the instruments name the subject and that a
coverage or robustness item had a claim. Three were surveyed before one was taken:

- **The ledger's `partial` rows.** `git blame` over `ledger.toml`, ordered by the commit that last
  wrote each `note = ` line, puts the least-recently-read band around the middle of this tree's
  history rather than at its start — the oldest eight were all rewritten in one session and each
  reads clean today. The twelve committed sweeps were run over the pristine tree and every hit was
  a shape their own catalogues describe as noise.
- **The oracle's ambiguous ink tail.** Its head is diagnosed by name and its top contradicted
  entries are the `DeviceCMYK` press pages, the JBIG2 fixtures and the shared-FreeType pair session
  780 classified.
- **The corpus gate's own three-class partition of `incomplete`** (ADR 0730). The class the gate
  attributes to **this reader** held exactly two mechanisms, and one of them is `doc/todo/22`'s
  Arabic free-text value, which is read, priced and pinned. So the other was the only unpinned item
  any instrument attributes to this program, and it was named rather than chosen: **a stroke whose
  colour is a tiling pattern**.

## The finding, in one sentence

§8.7.3's `partial` blocker — the outline is the backends' to compute, so tiling it here would be a
fourth stroke expander in the one crate that has none — is three true sentences about **one**
construction, and §11.5.2 gives a second: a group holding the stroke alone, taken for its alpha, is
the same region, and it travels as a `Command::Stroke` each backend expands with the expander it
already has.

## What changed

`Interpreter::tile` takes a region rather than a fill rule. `Tiled::Fill` is the old path,
unchanged; `Tiled::Stroke` builds an alpha soft mask over the stroke and lets the tiles carry the
state's own clip. Three corrections came with it: the tile span is `stroked_bounds` asked in device
space and mapped into the pattern's, the alpha constant is §11.6.4.4's `CA` rather than `ca`, and
§11.7.5.2's sixth condition now fails through a shape mask instead of being assumed away. Both
routes tile — the path's and §9.3.6's stroking text modes — and each has a test that fails without
it. No backend changed and no display-list type changed.

## What the gates said

The whole of `doc/todo/02` §2 ran after the final edit, against a pristine baseline taken in this
worktree before any edit. `fmt` clean, `clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"` clean, `nextest` green over the whole workspace, the doctest green, the
fuzz check green, and `cargo test -p conformance` green after the ledger row and the two new
quotations were fixed to the standard's own words — the quotation checker caught both, which is
what it is for.

The corpus gate's incomplete count falls by one and the *this reader* class falls to one mechanism.
The oracle's seven verdict counts are identical and the only per-page line that differs is
`scorecard_reduced.pdf` moving from `agrees (incomplete)` to `agrees`. quorra's cross-backend gate
reports the same differing set with the same figures. `doc/todo/00` step 7 was re-run over every
ambiguous page — the rule for any round that changes what is drawn — and its head reproduces entry
for entry.

The §4 sweeps were diffed before and after over the same twelve binaries: every delta is a witness
path that moved with a line number, §8.7.3's row now naming `/TilingType` and `Command::Stroke` as
its debt where before it named no identifier at all, and four new document quotations and two new
ledger ones, all six verbatim.

## The page

Trap 1's rule, and it is the whole of the round's evidence. `scorecard_reduced.pdf` at 2×, before
and after: blank where a dotted leader belongs, then the leader. `poppler` draws the same leader —
evidence that §8.7.2 was read right, never the reason it was read.

## What is left

§8.7.3 stays `partial` and its reason is now its subclause's, Table 74's `/TilingType`. The
construction costs one soft-mask buffer per patterned stroke, which is written down in ADR 0735 as
the price of not having a fourth stroke expander rather than left as an unexplained cost.
