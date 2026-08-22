# 657 — The parents that stopped reading their families

Five rows off the top of the blame ordering, four defects, and every one of the four is the same
shape: a parent row summarising a family it had stopped reading, with the right answer standing in
a child row all along. One of the four had been contradicted by a *sibling row corrected two hundred
and twenty-eight sessions earlier*, and one was contradicted by its own commit.

Date: 2026-08-22.
ADR: [0485](../adr/0485-the-sweeps-level-reviews-the-sentence.md).

Touched: `doc/conformance/ledger.toml` (§8.11.4.1, §9.7, §10.7, §11.7, §12.8.4.2),
`doc/todo/01-ledger-partial-rows.md`, the ADR and this file. No code.

## The order the three instruments gave

**The eighteenth sweep first**, as `doc/todo/01` says. `--bin overstated` printed **8 contradictions
over 170 parent rows asserting 125 terms**, 7 of them marked. Nothing new: the eight are 645's and
652's, verdict for verdict. Its one unmarked hit is §12.7's `/AP` against §12.7.5.5's "Table 236's
`/P` is deliberately not read here" — read this round rather than taken on 652's word, and it is
noise for the reason the module doc gives: the parent says a field's appearance is read from `/AP`
where §12.7.4.3's regeneration does not reach it, the child says Table 236's `/P` is a statement
about what invalidates a *signature* and is §12.8.2.2's question. Two different entries, no
contradiction. A fifth of a second, and it is still the right thing to run first.

**Then the blame ordering**, re-derived rather than taken (616's rule): 859 commits, **240**
`partial`-or-`reported` rows with a blamed note. 652 left a prediction — §11.7 at rank 4, §10.7.5 at
6, §10.7 at 7, §8.6.5.7 at 8, then a cluster of six at 9 — and it came out **exactly**, shifted up by
the five rows 652 read: §11.7 at 1, §10.7.5 at 2, §10.7 at 3, §8.6.5.7 at 4, and §12.8.4.2, §7.6.4,
§7.6.4.4, §8.11.4.1 and §9.7 sharing rank 5–9 on one commit. Read: ranks **1, 3, 5, 8 and 9**.

620's rule chose within the band for the eighth time running and chose right four times out of five:
§10.7, §11.7, §9.7 and §8.11.4.1 all state their reason as a claim about *this codebase*, and all
four were wrong about it. §10.7.5 and §8.6.5.7 were passed over precisely because their reasons are
readings of the standard — both are long, both are freshly tended, and neither has a sentence a grep
of the tree could falsify.

**Then enumeration**, and this time it paid rather than bounding. Walking `content/ext_gstate.rs`'s
Table 57 arms — the keys that file actually matches are `/LW /LC /LJ /ML /D /RI /SA /BM /CA /ca /TK
/AIS /BG /BG2 /UCR /UCR2 /TR /TR2 /SM` and the black-point names — against every ledger sentence
denying one of them found §10.7's, and finding it *that* way is what made the third instance legible
as a third instance rather than as a fresh mistake.

## The four defects

**§10.7 counted §10.7.3 among the parameters that are ignored, and `/SM` moves pixels.** The row
said "two parameters the clause lets a processor ignore and which are ignored (§10.7.2, §10.7.3)".
Flatness is genuinely ignored, by the clause's own permission. Smoothness is not: Table 57's `/SM`
has been read into the graphics state since the **seventy-fourth** session and decides how finely a
shading's colour function is sampled — `pdf_render::shading::Ramp::resolution_for`, honouring a
request finer than this device's 1/256 up to 4096 samples and keeping ours against a coarser one on
§10.7.3's own "each output device may have internal limits on the maximum and minimum tolerances
attainable". §10.7.3's row has read `implemented` on exactly that ground the whole time.

**And this is the second place the same sentence was written.** §8.4.5's row carried `/SM` on its
not-read list as "the silence recorded under §10.7.3", and the **five-hundred-and-sixty-fifth**
corrected it there — recording, in `doc/todo/01`'s own band table, that "a not-read list is a list
of claims, and the row that gets corrected when a mechanism arrives is the mechanism's own row
rather than the list that mentions it". That round fixed the list it was reading and did not grep
for the others. ADR 0101's shape: the claim was false from the **seventy-fourth** session, corrected
in one of its two homes in the **five-hundred-and-sixty-fifth**, and survived ninety-two sessions in
the other — **a retired claim is a string, and the round that retires one owes a grep of the tree
rather than of the family it happens to be in.** `--bin retired -- smoothness` now prints 18
mentions across the tree and not one of them is a surviving denial.

The row's evidence was file-only — two whole files — and now names
`pdf_render::shading::a_smoothness_tolerance_only_ever_asks_for_more_samples`, which is the test
that fails if `/SM` stops moving the sampling.

**§11.7 attributed the whole of §11.7.5's debt to §11.7.5.3, and there are two.** The row read
"the other three are `partial`, each naming what it owes: §11.7.2's CIE-based `should` and Annex P,
§11.7.4.4's one drawable-shape condition, and §11.7.5.3's black generation". §11.7.5 owes a second
thing and it is the loud one: **§11.7.5.2 has been `reported` since the six-hundred-and-thirty-seventh
session** — `Unsupported::TransferFunction`, raised on a mark this clause does not call fully opaque
made while some mark on the page has carried a transfer function. That row was `inapplicable` on a
reading of the clause that turned out to be wrong (its closing sentence puts the *page's default*
transfer function on every point whose topmost object is not fully opaque, and needs no second
function to bite), went `silent` in the six-hundred-and-thirty-second and `reported` five rounds
later. The parent that summarises the family never heard, on either move.

That row's `test` array was file-only too, for a claim that now includes a report; it names
`transfer_functions.rs::a_translucent_mark_under_a_transfer_function_is_reported` and
`content/transparency.rs` beside them.

**§9.7 named "§9.7.5.1's remainder", and §9.7.5.1 went `implemented` in the same commit.** `git log
-S "§9.7.5.1's remainder"` returns exactly one commit, `c1c9e621` — the four-hundred-and-twenty-ninth
session, the fourth-sweep round — and that commit's own message reads "And §9.7.5.1 was partial above
a note naming nothing owed, which breaks the ledger's own definition of the status". So one commit
moved the child to `implemented` and wrote the parent's sentence saying the child still owes
something. It stood for 228 sessions. §9.7.5's row reached the same answer from below in the
six-hundred-and-fifty-second — "§9.7.5.4's c) is the whole of it" — leaving the grandparent as the
last place the retired debt was written.

**620's shape five, checked and passed**: §9.7's one cited test does reach its claim. The row says a
Type 0 font is two independent mappings and both are read;
`the_cid_widths_agree_with_the_font_programs_own_advances` checks both **without consulting either**,
by holding `/W`'s width per CID against the embedded program's `hmtx` advance per glyph — two
structures written by the same producer and travelling separately, which agree only if §9.7.6.2's
code-to-CID and §9.7.4.2's CID-to-glyph both landed where the producer meant. Written into the note.

**§8.11.4.1 named two of its three `partial` children, and its own parent names all three.** The
missing one is §8.11.4.4: Table 100's `User` and `Language` categories, which are questions about
this processor rather than about the document and are answered `Recommendation::Unanswerable` rather
than by the clause's "otherwise OFF". §8.11.4's row — one row above, the parent of this one — has
named all three since the same four-hundred-and-thirty-seventh session that put a debt in this note
at all. The §8.11.4.5 half was understated too: the row named the zoom reapplication and not the
Print and Export events whose changes "persist only for the duration" of an operation this program
does not perform.

## The kept row

**§12.8.4.2**, rank 5. `partial` because §12.8.4's `shall`s are addressed to a *validator* — collect
the certificates, CRLs and OCSP responses relevant to validating the signature; where a timestamp
token is present and valid, use its UTC time as the reference for checking revocation status up to a
trusted root — "and what this tree does instead is count the material and say so". The check owed was
620's shape five, and the one cited test reaches it exactly:
`a_documents_own_declaration_can_be_held_against_it` states a `/DSS` with two `/Certs`, one `/OCSPs`
and one `/VRI` entry and asserts `security_store`'s four counts back. That is the whole of the claim
and no part of the validation. Written in, with the confirmation.

## `spec-errata`, and a filing one clause off for the third round running

`emit` over all fourteen documents before writing. **Nothing at all under §10.7, §10.7.3, §11.7,
§9.7 or §8.11.4**; §12.8.4.2 carries Issue #448, which inserts "was added in PDF 2.0 and" — a version
note that bears on nothing here.

**Issue #686 prints under `## 8.11.4.1` and belongs to §8.11.3.2**, and this one is settled by the
annotation's own text rather than by a `/Rect`: the issue's p. 298 half strikes a bare "…" and writes
"... endstream endobj  10 0 obj", which is the continuation of its *own* p. 297 half under §8.11.3.2
— both frame that clause's example in `9 0 obj << /Length ... >> stream`. `pdftotext -layout -f 298`
confirms there is exactly one "…" on the page and it is in the example at the top; §8.11.4.1's body
text has none. `emit` files by the page, the example ran over a page break, and the heading that
opens the far side got the annotation. Recorded in §8.11.4.1's row so nobody redoes it.

Errata Collection 3's Issue #371 rewrites §10.7.2's "It shall be a positive number" into a 0-to-100
range with 0 meaning the device default. It touches neither the permission that row rests on nor
anything this tree does, since `i` is read and discarded either way; noted here rather than in the
row, which was not read this round.

## The price re-derived

One price was re-derived and it held. §10.7.5's row prices the first of that clause's two
requirements — grid-fitting a stroke's *coordinates* — as a departure of §10.7.4's family, licensed
by §10.7.1's NOTE, with nothing to report because "there is no page on which this device could do
better". The cheapest re-derivation is `doc/habits.md`'s own: ask what the libraries and layers
already contain. `pdf_render::sub_pixel` and `pdf_render::collapsed` were both built after that
price was written, and both **snap nothing** — a sub-pixel rule keeps the fractional position the
document stated and divides its coverage between two rows, a collapsed fill takes the row it lies
in — so the library's pieces do not shorten the work, they make it a different work. The price
stands and the row already says why in more words than this. Not a finding; a re-derivation that
came back the same, which is what the habit asks for either way.

## The instrument this round leaves behind

Not a sweep — a way of reading the twelve that exist. The first correction to §8.11.4.1 named Table
98's `/Configs` and a configuration's `/Name` and `/Creator`, every word of it true, and `--bin
unread` went from its standing **69 rows / 182 keys** to **70 / 185**: the three new keys are the
three §8.11.4.3's row already carries, and all three are that sweep's dominant noise shape
(`/Configs` witnessed only by `examples/oc_usage_census`, `/Name` colliding with a *group*'s,
`/Creator` being `metadata.rs`'s §14.3.3 entry). Rewritten to point at §8.11.4.3's row instead of
repeating its list, the sweep is back to 69 / 182.

652 found the same instrument reviewing its draft and drew the lesson about *vocabulary*. The
lesson is about the **level**: one integer per sweep, stable across rounds that change nothing,
produced by a program that does not know what this round is trying to say — and a round can talk
itself past a hit but cannot talk `unread` from 185 keys back to 182. ADR 0485, with the delta table
this round owed and the reason it is a habit rather than a gate.

## Gates

The change is **documents only**, so `doc/todo/02` §2's map asks for the core four lines, the
conformance gate and the sweeps a moved ledger owes; the parent round named exactly that set plus
the fuzz line. `tools/round.sh` called this a fifth round on a session count of 655, which is 655
and 656 not yet being merged.

`cargo fmt --all --check` exit 0. `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`
exit 0. `cargo nextest run --workspace` **2390 passed / 17 skipped**. `cargo test --workspace --doc`
clean, every result line `ok`. `RUSTFLAGS="-D warnings" cargo check --manifest-path fuzz/Cargo.toml
--bins` exit 0. `cargo test -p conformance` green — **875 rows**, and the status breakdown is
unchanged at 436 implemented, 222 partial, 18 reported, 78 inapplicable, 8 writer-side, 113
out-of-scope. **No `silent` row**, 0 unreviewed. No status moved, which is right: all four defects
are a parent misdescribing a debt it still has.

All twelve committed sweeps run, before the edit and after it, with the deltas accounted for in
ADR 0485 §4. `overstated` 8 contradictions with 7 marked, unchanged; `counts` 4 places counting one
family twice, unchanged; `quotations` 1 diverging ledger quotation, the standing §8.4.4 one;
`unread` 69 / 182, unchanged; `tables` 6 denials the table contradicts; `pointers` 118 absent and 13
undefined symbols, none of them this round's; `blockers`, `capabilities`, `entries`, `inapplicable`,
`owed` and `callers` printed their standing populations and no new hit.

**Overlap with the parallel rounds: none seen.** 655, 656 and 658 were briefed to touch a row or two;
nothing this round wrote is outside §8.11.4.1, §9.7, §10.7, §11.7 and §12.8.4.2.
