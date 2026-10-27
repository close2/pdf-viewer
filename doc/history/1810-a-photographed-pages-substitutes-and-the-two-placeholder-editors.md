# 1810 — A photographed page's substitutes are the fonts the output holds, and the two placeholder editors take the replacement's decision

Session 1810, Slot 5 of batch one hundred and twenty-one, an archive round; 2026-10-27. ADRs 2454
and 2455; Q676 unused. I worked in `scratchpad/r1810/`. No ledger status moves.
**Premises, held on plants.** ADR 2445's plant — a Helvetica drawing a space beside the unnamed
`Adobe-Identity` font — reported Helvetica embedded while the output held none; and a catalog
packet of two `rdf:RDF` elements stating a malformed `pdfaid:amd`, and one stating a structure field
its value type leaves out, each refused its row with `prepare::NOT_ASKED_FOR`.
**Built (ADR 2454).** `SubstitutedFont::font` names the font each entry substituted, in the form the
photograph's deferred fonts use (`fonts::selected`, shared). Once a page is photographed,
`photographed::still_held` walks the source's objects from `/Root` and `/Info` with each photographed
page's `/Contents`, `/Resources` and `/Group` left out, and the report, the losses and the
`xmpMM:History` entry keep only fonts it reaches; reachability rather than selection, so a font an
annotation, another page or an inherited dictionary still names stays named.
**Built (ADR 2455).** `prepare_amendment` and `prepare_field_descriptions` skip a stream the fresh
packet replaces and name it where the row failed there; `met_by_the_replacement` (ADR 2442's
function, widened to three rows) gives each the packet row's decision. **A premise under it did not
hold**: ADR 1245 says a fresh catalog packet gains the identification schema, but it was restated
only where a row asked, so a source whose packet already stated its identification converted into a
file failing section 6.6.4's part and conformance rows, and was refused by the output check. The
schema is now restated wherever the catalog's packet is composed afresh.
**Proof.** Six tests: the plant and its control (a `Square` annotation's appearance on the page
showing the same Helvetica keeps it named, its program in the output); the amendment and the field
rows taking the replacement's decision authorised and not, and under both shipped profiles; and a
replaced packet that stated only its identification stating it again. Each failed before its change
(the last with the restatement planted out); veraPDF 1.30.3 passes all five 2b plant outputs.
**Census** (converted/conforming/refused/unreadable), HEAD → this, all eight lines unchanged: pdf.js
`as-if-printed` 2b 789/2/162/11, 4 792/0/161/11, `as-shown` 2b 764/2/187/11, 4 768/0/185/11;
`doc/corpora` `as-if-printed` 2b 350/6/115/16, 4 352/0/120/16, `as-shown` 2b 303/6/163/16, 4
303/0/169/16; every one of HEAD's 919 census lines is in this run's output. No output changed, so
no veraPDF run is owed on a corpus output. `cargo tree -i pdf-transform` reaches neither
`render-raster` nor any crate of the corpus binary, so against HEAD's six arms nothing can move.
**Gates.** In an export of HEAD and my hunks (a sibling's mid-edit broke the worktree's build):
`rustfmt --check` on my ten files and `cargo fmt --check -p pdf-transform` exit 0; clippy `-D
warnings --all-targets` on pdf-transform exit 0; nextest 722 passed, 0 failed; rustdoc `-D warnings`
on pdf-transform, with and without private items, exit 0; `tools/state.sh --round 1810 archive` exit
0 (validator missed 0, over 0 at six targets; every conforming input still conforming after
conversion; cross-check 13 560 compared, 0 disagreements); census `--tree 6` exit 0 at HEAD and
here. `cargo test -p conformance --no-fail-fast` in the worktree exit 101, two reds and neither
mine (a `pdf-model` doc path, 1813's record length). 7000 s.
