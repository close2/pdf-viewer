# 1072 — the folding the clause asks for, and the layout nobody asked

Date: 2026-09-15. ADR 1086. Seven §12.3 rows read, four moved. Touched `data/unicode/`,
`pdf-model/{build.rs, case.rs, collection.rs}`, `viewer-host/panel.rs`, `chrome.rs`, three tests,
`/NOTICE`, the ledger.

**§12.3.5.2 — case normalization is the annex's function now, not one that looks like it.** The
clause sends the comparison of two names in one folder to Unicode Standard Annex #21, whose caseless
matching is toCasefold; `case_normalised` called `char::to_lowercase`. They disagree on the
characters folding to a different string from the one they lowercase to — U+00DF to ss, U+017F to s,
U+03C2 to the sigma U+03A3 folds to — and there the lowercasing answer is *no collision*, so the
restriction went unreported and the panel called the document conforming.
`data/unicode/CaseFolding.txt` is committed beside the CMaps, and `crates/pdf-model/build.rs`
compiles its status C and F rows — the file's own *full* folding — into a sorted `static`
`pdf_model::case` binary-searches, so nothing is parsed at startup. Status S would rebuild the
defect; status T, the Turkic pair, is excluded on the file's stated default, because a folder name
carries no language that could ask for it. ADR 1086 is the decision.
**Calibrated by planting the defect (trap 13)**: `case`'s tests compute the lowercasing answer
beside the folding one for all three characters, so each assertion names a collision the old
function missed rather than checking the table against itself, and
`two_names_that_differ_only_by_a_sharp_s_are_both_reported` takes the pair through
`Collection::read` from a document's bytes. Row → `implemented`.

**§12.3.6 — the selection rule was implemented, reachable, and asked by nobody.** Every `shall`
here is a producer's; the processor's sentences are `should`s, and the one no code obeyed was
"use the value of the Layout entry to present the collection". `panel::DRAWN_LAYOUTS` is now the
capability `Navigator::preferred` takes — `Tree`, which `collection_rows` builds by construction,
`D`, and `H` — asked by both row builders, so all three windows select the same layout. The four
this panel is not (`T`, `FilmStrip`, `FreeForm`, `Linear`) are named out loud by
`panel::unsupported_presentation` instead of silently replaced, and so is Table 153's `/View T`.
Row → `implemented`. **The corpus states the case**: `digitally_signed_3D_Portfolio.pdf` asks for
`/View C` and states Adobe's SWF navigator, which holds none of Table 160's entries.

**§12.3.2.2 and §12.3.2 — a row was carrying another row's debt.** Every requirement §12.3.2.2 puts
on a reader is executed; what was left is a page number in a file this program declines to open, and
that refusal is §12.6.4.3's, whose row is `reported` and says what would close it. Both rows →
`implemented`. §12.3.5 and §12.3.5.1 stay `partial`, their notes now separating surfaces no
window has (tile mode, colours, splitter, four layouts), reported since today, from `/Sort`'s order
— this family's one silence, because the values it sorts by are §7.11.6's collection item and no
host is handed one. Named there for a round to take.
