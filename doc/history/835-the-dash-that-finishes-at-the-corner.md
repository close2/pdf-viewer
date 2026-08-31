# 835 — The dash that finishes at the corner, and the vertex no dasher was asked about

Date: 2026-08-31. On `main` directly, from `33a18e64`.

ADR: 0762 — the dash that finishes at the corner.

Touched: `crates/pdf-render/src/closing.rs` (new, the rule and its eight unit tests),
`crates/pdf-render/src/lib.rs` (the module and its export), `crates/render-cpu/src/lib.rs`,
`crates/render-gpu/src/scene.rs`, `crates/render-quorra/src/stroke.rs` (three call sites),
`crates/render-quorra/tests/dashed_close.rs` (new, the cross-backend gate),
`doc/conformance/ledger.toml` (§8.4.3.4 and §8.4.3.6, both notes corrected),
`doc/todo/03-more-corpora.md` (§28, the chunk), `doc/adr/0762-*`, this file.

## The round

`doc/todo/03` asks for a chunk of an unwalked corpus. There is not one: the SafeDocs crawl is
65 944 of 65 944 ranked over nine chunks, the four submodule corpora are ranked, and what remains
in that file for a *population* is a 31 GB download. The oracle gate is green, its undiagnosed
queue is empty and each of its seven verdicts is held by name, so the fallback — the worst standing
page — had no unexplained head either: the contradicted list's own head is `xobject-image.pdf` at
127.75× and it has carried a diagnosis since ADR 0349.

So the chunk was the item §4 and §14 of that file both name and nobody had taken: **`pdf-differences`
read as clauses.** Its eighteen cases each quote a normative sentence and publish the picture that
sentence requires; all 37 documents were rendered and read against the clause rather than against
the picture beside it. Fourteen agree — including ISO 32000-2's corrected ColorBurn and ColorDodge
edge cases, §8.6.6.3's out-of-range indices, §8.5.3.2's degenerate line caps, §11.7.4.4's atomic
fill-and-stroke, and §8.4.3.6's negative dash phase, which was measured rather than eyeballed
because the corpus's own wrong picture differs from its right one only in which way a slope leans.

**One case is a defect of this tree, and it was on all three rasterisers.** §8.4.3.4 joins the last
dash of a closed subpath to the first only when the last "ends *within* an on-dash"; §8.4.3.6 says
that where "the end of a dashed segment coincides exactly with a join point, then the end cap is
painted before the corner". Every dasher this tree draws through merges the two whenever both are
on, which reads the first sentence without its adverb. `DegenerateDashing.pdf` states both cases in
one file — a 200 × 45 rectangle whose perimeter of 490 finishes an on-dash exactly at the lower-left
corner, and a 200 × 44 one whose 488 stops eight units inside one — and this tree drew the
document's round join on both.

`pdf_render::opened_where_a_dash_ends_at_the_close` replaces such a subpath's `Close` with the
straight segment Table 58 defines it as, which leaves the geometry alone and gives the stroker two
ends to cap. All three backends call it, for `degenerate.rs`'s reason. Measured with the rule turned
off, the quadrant §8.4.3.6 leaves empty held 3.133 square units from the processor, 3.086 from
quorra and 2.753 from vello against a quarter disc of π — one wrong answer from three libraries,
which is what put the rule in the shared crate rather than in a backend.

The comparison is exact and the `#[expect]` above it says why: the clause's own condition is a
coincidence, a margin here would be one nobody derived, and it would swallow the neighbouring case,
which is the one §8.4.3.4 joins. A subpath holding a cubic is left as written, because a Bézier's
arc length has no closed form and the coincidence cannot be established for one.

## What was found in the ledger, and it is the second track

Both clauses were `implemented` and both notes were wrong about this tree. §8.4.3.4's said the
joining sentence was one "which both dashers do" — two false claims in four words, since they do not
do it as stated and there are three of them rather than two, which is `--bin parts`' own subject.
§8.4.3.6's said "[t]he per-subpath restart and the cap and join treatment are the rasteriser's, and
both backends do it", which is true of every vertex but the one this round is about. Both are
rewritten to what is true, and the `parts` sweep no longer names either row.

## Gates

The full §2 sequence, on a quiet machine: formatting, `clippy` under `RUSTFLAGS="-D warnings"`,
2830 workspace tests, the doctests, both `fuzz/` lines, and every gate line. The oracle's 970
non-agreeing page lines are **byte-identical** either side of the change, measured by re-running it
with the diff reversed — which is the expected result and not a weak one, since the construction
needs a closed dashed subpath whose perimeter lands exactly on a dash boundary and no document of
the gated populations states one. `render-quorra`'s corpus gate, `fixed_documents` (41 rows, 0
absent) and `cargo test -p conformance` are green; §5's binaries were rebuilt and installed after
the change rather than before it.

Three sweeps were run beside the sequence rather than during it — `pointers`, `quotations` and
`parts` — and none of them names anything this round added.

## What the next round might take

`doc/todo/03` §28 leaves one gate rather than a population: `IndexedCS_negative_and_high.pdf` and
`InlineAbbreviations.pdf` each have an expected value a clause supplies with no reference in it —
§8.6.6.3 makes the top row of patches identical to the bottom row, and §8.9.7's Tables 91 and 92
make all eight images identical — and both are one-line assertions over a document already on this
disk. `VerticalText.pdf` is the other thing that chunk leaves, and it is `doc/todo/21` §3's rather
than that file's: `/Identity-V` over a non-embedded `Adobe-Japan1` `CIDFontType0`, where the
producer has already chosen the vertical-form CIDs and a substitute reached through Unicode draws
the horizontal glyphs.
