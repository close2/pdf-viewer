# 0980 — Four measurements that became walks, and a datum with one reader fewer

Status: accepted. Session 969.
Context: `crates/pdf-model/src/icc.rs`, `crates/pdf-font/src/predefined.rs`,
`crates/pdf-font/src/standard.rs`, `crates/pdf-font/src/standard_metrics.rs`,
ISO 32000-2 §8.6.5.5, §9.6.2.2, §9.7.5.2, Annex D.5, Annex D.6,
`doc/traps/instruments-and-reports.md` traps 11, 13 and 25, ADRs 0963, 0970 and 0971.

## What was outstanding

ADR 0971 left a census table for `data/` with seven rows and three of them asserted. Its own
closing sentence named the debt:

> The census table above is the round's standing answer for `data/`, and two rows of it are
> assertions rather than measurements taken once — `cmap.rs`'s and `tounicode.rs`'s. The other
> four are measurements in this document, which is the weaker form: a round that adds a datum to
> `data/` has nothing that fails.

That is `CLAUDE.md`'s own rule about where knowledge lives, applied to a number: **a fact that can
be counted is not written down; what is written down is the command that counts it.** A census row
in an ADR is a number written down. This round turned the four into walks and then ran ADR 0971's
second outstanding item — the two-readers sweep — over `data/standard-fonts/`.

## The four walks

Each follows the shape `no_registered_cmap_is_cut_by_these_bounds` established: walk the whole
carried population against the bound, then **follow one datum through to the value it decides**,
so that a bound raised only far enough to stop a flag firing while the read still lost the tail
fails too. Each was calibrated against the defect it looks for, per trap 13.

**`icc.rs::no_bound_in_this_module_cuts_the_shipped_profile`.** `data/icc/sRGB2014.icc` is 3 024
bytes against `MAX_PROFILE`'s 16 MiB, states 16 tags against `MAX_TAGS`' 1 024, and its longest
text tag is 45 characters against `MAX_TAG_TEXT`'s 4 096. Every one of those bounds answers `None`
rather than reporting, so a later edition of the ICC's own file crossing one would be read short in
silence. The walk asserts the header's size field describes the whole file, the two
margins that can be stated, the file's own tag table (every tag inside the profile, the widest
`bTRC`'s 2 060-byte tone curve) and then the two strings the tag reader produces — which is what a
bound cutting the text would lose. Calibrated three ways: `MAX_PROFILE` to 1 KiB, `MAX_TAGS` to 8
and `MAX_TAG_TEXT` to 8 each fail it, and the third fails on the *string*, which is the second half
doing its work. **`MAX_TAG_TEXT` deliberately has no margin assertion**: a text past it makes
`ascii_text` answer `None`, so `text.len() <= MAX_TAG_TEXT` beside the string assertions could
never fail, and an assertion that cannot fail is worse than none — it reads as coverage.

**`MAX_CLUT`, `MAX_INPUTS` and `MAX_OUTPUTS` are deliberately not asserted, and saying so is part
of the row.** They bound a colour lookup table and this profile is the matrix-and-curve form —
`a_tag_table_is_searched_by_signature` already asserts it carries no `A2B0`. A carried datum that
never reaches a bound proves nothing about it, and a walk that pretended otherwise would be trap
25's shape: an assertion over a population that does not exist, passing because it found nothing.

**`predefined.rs::no_carried_cmap_chain_is_cut_by_the_depth_bound`.** This is the third bound the
239 carried `CMap` files pass through, after `cmap.rs`'s four and `tounicode.rs`'s three, and it is
the one whose silence is widest: `resolve` and `resolve_unicode` stop a `usecmap` chain by
answering `None` for the *base*, and `CMap::parse(&source, None)` then builds a map missing every
code the base stated, with §9.7.6.2's lookup answering nothing for them. The embedded form refuses
out loud — `composite.rs`'s `read_cmap` returns `UnsupportedEncoding` — and this one cannot, because
it has no document to blame.

What makes the silence safe is arithmetic: 80 of the 239 files name a base, the deepest chain is
**two** (`ETenms-B5-V` → `ETenms-B5-H` → `ETen-B5-H`) against a bound of four, and **no file names
a base this binary does not carry** — which nothing had ever asked and which would produce exactly
the same base-less map. The second half follows the deepest chain to `<c6e0>`, a line of
`data/cmaps/ETen-B5-H` that neither file above it restates, and to `<20>`, which `ETenms-B5-H`
*does* restate over its base, so the order of the chain is checked as well as its length.
Calibrated twice: `MAX_DEPTH` at 1 fails naming `ETenms-B5-V`, and with the bound left alone but
the resolved base thrown away — a chain counted right and lost anyway — the second half fails
instead.

**`standard.rs::every_name_annex_d_states_has_a_glyph_in_the_face_carried_for_it`.** Table D.5
assigns 189 of the 256 codes and Table D.6 assigns 188, and every one of those names has a glyph in
`FoxitSymbol` and `FoxitDingbats` respectively. This is the one row where a carried *table* and a
carried *face* have to agree, and there is no second route: `substituted.rs`'s bare-CFF path looks
the name up in the program's charset precisely because `a1` and `a192` are in no Unicode mapping
worth trusting. The faces hold one and fourteen glyphs besides `.notdef` that the tables do not
name, which is the direction that costs nothing. Calibrated by deleting `universal` from the
charset (the first half fails, naming it) and by making the code table refuse that one name while
the charset still holds it (only the second half fails).

**`standard_metrics.rs::every_latin_1_name_these_tables_state_has_a_glyph_in_every_carried_face`.**
The fourth row is the one whose measurement is *not* "nothing is missing": 86 of `Times-Roman`'s
315 names and 84 of `Courier`'s reach no glyph in the Foxit faces. ADR 0971 recorded that and
recorded why it is not a defect — ADR 0270's wider-face search replaces such a face where a
document's codes need it, and `content/text.rs` counts a code that reached no glyph — but a list of
86 names is not something to assert, and asserting the *count* would be a ratchet over a number
with no meaning.

What is assertable is the **line between the two halves**, and it is structural: a name whose Adobe
Glyph List character is U+00FF or below has a glyph in every one of the fourteen, and every name
that has none is above it or is not in that list at all. 189 of the 315 Latin names are inside
Latin-1, 39 of `Symbol`'s 190 and exactly one of `ZapfDingbats`' 202 — its `space`, the rest being
`a1` to `a202`. A face swapped for one that lost `eacute` fails; one that gained `Abreve` does not.
`commaaccent` is the single name the list gives no character at all, which is why Liberation Sans
has one absence of its own and why the line is drawn at *the list*, not at the codepoint alone.

The second half here is two assertions rather than one, because a 189-name population admits a
failure a single lookup would not see: the names reach **distinct** glyphs, and every one of them
draws contours bar the space. Calibrated by deleting `eacute` from the charset (the first half
fails) and by building the name table with every value zero (the distinctness half fails, naming
189 names sharing one glyph); the all-zero plant is the one the "does it have the name" question
cannot see.

**No bound was raised, and none of the four is close to one.** That is the same discipline ADR 0971
applied at 81% of `MAX_RANGES`: the defect these walks exist for is the silence, never the margin.

## The two-readers sweep on `data/standard-fonts/`

ADR 0971's habit — *when one datum has two readers, a census built for one of them proves nothing
about the other* — named the next datum to ask it of and said it had three readers, `sfnt`, `cff`
and `type1`, with the tables describing the faces in a fourth place. Run properly, the answer is
different in both directions.

| reader of `data/standard-fonts/` | what it is | asserted by |
|---|---|---|
| `cff.rs`'s `CodeToGlyph::read` | the ten Foxit bare CFF faces, by glyph name | this round's two font walks; `every_compiled_in_face_parses` |
| `cff.rs`'s `with_advances` | the same ten, for a converter restating a width | `restate.rs::an_outline_survives_its_advance_being_restated`, every glyph of all twenty faces |
| skrifa's `FontRef` (`sfnt.rs`'s format) | the four Liberation Sans faces, by character | this round's metrics walk; `every_compiled_in_face_winds_its_contours_the_same_way` |
| `encoding.rs`'s Tables D.5 and D.6 | the tables that *encode* two of the faces | this round's annex walk |
| `standard_metrics.rs`'s ten width tables | the tables that *measure* all fourteen | this round's metrics walk |
| `viewer-ui/src/chrome.rs` | the program's own panel text, through `LoadedFont::standard` | loud by construction: `Chrome::set`'s last arm draws a placeholder box, and `pdf-model --example interface_font_census` counts them (ADR 0326) |
| `viewer-ui/tests/notices.rs` | `SHA256SUMS` and the two licence files beside the faces | `notices.rs`'s own walk |
| `pdf-transform`'s `shipped_face` | the converter embedding a face | `FontError::NoSubstitute` where the face draws none of the declared codes |
| **`type1.rs`** | **nothing** | see below |

**`type1.rs` is not a reader of this datum**, and the habit's own grep is what finds it: the ten
files are named `.pfb` and are *bare CFF programs*, declared `Format::BareCff` in `standard.rs`, so
nothing ever hands them to the Type 1 parser. The habit named three readers and one of them is a
file extension. `every_compiled_in_face_parses` now asserts it — `type1::Program::parse` rejects
every one of the fourteen — which is the assertion a file swapped for a real Type 1 program without
its `Format` changing would fail. Per trap 25 the refusal is checked for vacancy the other way:
`crates/pdf-model/tests/type1.rs` parses the corpus's embedded programs through the same entry
point, so the parser is not simply refusing everything.

**And there are two readers the habit did not name**, both outside `pdf-font`: the program's own
chrome, and the licence walk. Neither is a gap — the first is loud, the second is asserted — but a
census that had stopped at the crate would have missed both. The lesson is the habit's own, one
level up: **the readers of a datum are not bounded by the crate that carries it.**

## What this round did not find, and the measurement that says so

`cff.rs` holds two bounds that a census of this datum has to account for, and only one of them
is reached by it. `calls_local_subr`'s `MAX_DEPTH` is asked only during CID-keyed Font DICT
repair, and no compiled-in face is CID-keyed, so the carried faces never put a question to it —
the same "a datum that never reaches a bound says nothing about it" the ICC row states, one crate
over.

`MAX_INLINE_DEPTH` *is* reached: it bounds how many subroutine calls the width restater follows
before giving up on a charstring. Measured over all fourteen compiled-in faces: the four Liberation
Sans faces, the two symbolic ones and `FoxitSerif` itself lose no glyph, and the other seven Latin
Foxit faces lose between two and six each — 6 of 244 at worst, against a test that tolerates 10%.
Raising `MAX_INLINE_DEPTH` from 8 to 64 in a scratch build changes **not one of them**. The bound
is not what stops them: `subroutine_body` declines a subroutine holding a `hintmask` after a nested
call, which is the case its own doc comment names, and `with_advances` turns it into
`CffError::AdvanceNotRestatable` with the glyph in it. Loud, and clear of the data. No walk was
added for a bound the carried data does not approach; the A/B is recorded here so the next round
measuring it does not have to.

## What this leaves

`data/` now has a walk under every bound that its own contents reach, and `data/standard-fonts/`
has a reader list with an assertion against each. The datum with no census of this kind left is
`pdf-spec`'s Arlington tables, which are generated rather than vendored and whose readers are a
different question again.

## A habit worth recording, which this round could not place

Session 972 owns `doc/habits.md` and `doc/traps/` this round, so this is here for the orchestrator
to place rather than written into either.

> **A datum's readers are not bounded by the crate that carries it, and one of the readers a
> census names may not be one.** `data/standard-fonts/` was said to have three readers inside
> `pdf-font`. It has two there — the third was a file *extension*, `.pfb` on ten bare CFF programs
> — and two more outside it, in `viewer-ui`: the program's own panel text and the licence walk over
> `SHA256SUMS`. The grep that gets this right is over the **path**, not over the module: everything
> that names the directory, in every crate, plus everything that names the constant the directory
> is compiled into. A reader list assembled from one crate's imports is a list of that crate's
> readers.

