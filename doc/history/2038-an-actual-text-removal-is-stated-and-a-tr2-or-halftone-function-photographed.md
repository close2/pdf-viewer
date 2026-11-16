# 2038 — An ActualText removal is stated, and a TR2 or a halftone's function photographed

Session 2038, an archive round in the audit worktree, on what ADRs 2888 and 2889 left of R03's finer remedies.
Rows 10.5 and 14.9.4 (notes); ADRs 2892 and 2893; Q865.

**The slice.** The brief's three, in its order: a statement-page sentence for `actual-text-private-use`; `/TR2`
other than `Default` and a halftone's `TransferFunction` under the photograph; the Type 3 width as a question.

**The premise.** All three unbuilt. The first two had effort as their only obstacle: removing both entries was
built (ADRs 1978, 2007) and only the photograph route was missing. The third is owner-blocked: no answer opens
ADR 0816's fence for a glyph procedure (A694 item 3 is R11's and R03a's), so it is Q865 and not built.

**Built.** `statement::TAKEN` and `config::STATED_GRAPHICS` take the `ActualText` row as a fourth one-entry row,
its value the source's string; `state-every-loss` answers it `try = ["append-statement", "discard"]`.
`fenced::ROWS` takes the `/TR2` and halftone rows, their pages those reaching a non-`Identity` function, the walk
following a state's `HT` and a type 5 halftone's components; `as-shown` keeps ADR 2394's gate. veraPDF found the
statement page's root `/K` written the source's structure element inline; `/K` now keeps the reference.

**Evidence.** veraPDF 1.30.3, on the tests' outputs: the `ActualText` statement compliant at 4; `/TR2` photographed
at 2b and 4, the halftone function at 2b, both `Identity` cases at 2b under both profiles, all compliant.

**Tests.** `tests/archive/finer_remedies.rs` (three), `tests/archive/statement.rs` (one). **Plant**: the two rows
out of `fenced::ROWS` and the `ActualText` entry out of `TAKEN` fail three of the four; HEAD's `tagged.rs` fails
the fourth.

**Gates.** Over `pdf-transform`: fmt exit 0; clippy `-D warnings --all-targets` exit 0; nextest 1209 of 1209;
rustdoc private exit 0; `cd fuzz && cargo check --bins` exit 0; conformance after `git add -N` exit 0; `pdf-archive`
corpus exit 0; `archive_corpus` exit 0 (8 of 8); the census with the pack exit 0, its lines and `rank.py`'s table as
at HEAD (R03 52 / 53). `corpus`, `raster_golden` not run: no crate they build changed. `state.sh docs` exit 0.

**Q865.** `issue19634` (1 page) and `issue19971` (3 of 6) are photographed for the Type 3 width alone; `303226`'s
19 pages are photographed for another font as well, so the remedy keeps nothing there.

**Left.** The validator reads `/ActualText` on objects and inline operands, not on a structure element written
directly inside another object. An `Identity` `TR2` or halftone function beside a photographed `/TR` is refused
(ADR 2893). A component omitting a required function is photographed and unpinned. Type 3 widths wait on Q865.
