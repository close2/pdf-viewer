# ADR 0029 — A composite font is two mappings, and the Identity case is where both vanish

Status: accepted, 2026-07-30.

## Context

A Type 0 font worked here only under `Identity-H` with an identity `/CIDToGIDMap`. Everything
else was refused and reported: 14 corpus fonts named an embedded `CMap` stream, 41 more carried
a `/CIDToGIDMap` that was not the name `Identity`, and between them **31 documents drew no text
at all** on page one. That was the largest gap of any kind on the demand list — the corpus gate
counted 100 fonts on its `Text` row, and these were most of it.

Reading §9.7 as a family is what shaped the work, and the first thing it settled is why the
Identity case had been enough for nineteen sessions. §9.7 describes **two independent
mappings**:

- **§9.7.6.2**, a code to a CID, through the `CMap` named by `/Encoding`.
- **§9.7.4.2**, a CID to a glyph index, through the `CIDFont`'s own program or its
  `/CIDToGIDMap`.

Under `Identity-H` with `/CIDToGIDMap /Identity` both are the identity, so *neither has to be
read* and a code can be taken straight to a glyph index. That is not a simplification of the
model; it is the one configuration in which the model is invisible. Almost every modern producer
emits it, which is why the tree got as far as it did.

Two things follow that are worth stating before the decisions. A `CMap` decides **how many bytes
the next code occupies**, not only what it means — so a code is not a number, it is a number and
a length, and §9.3.3's word spacing is stated in exactly those terms. And a composite font's
widths come from `/W`, which is indexed by **CID**; under Identity that is the same as the code,
which is why looking widths up by code had worked.

## Decision

### A `CMap` is its own type, and it answers both of §9.7.6.2's questions

`pdf-font/src/cmap.rs` holds `CMap` and `Code`. `Code` carries a value, a length in bytes, and
whether it matched a codespace range at all — because §9.7.6.3 treats a code matching none as
*invalid* and sends it straight to a substitute glyph rather than to the character mappings, and
nothing about the value distinguishes those two cases.

Every table in it is indexed by code length, because the clause is:

> The code extracted from the string shall be looked up in the character code mappings for codes
> of that length.

So the one-byte code `20` and the two-byte code `0020` are different codes that happen to share
a value, and a `CMap` mapping both is legal. `issue2931.pdf` declares a single one-byte
codespace range `<20> <76>`; `issue18117.pdf` declares four ranges of one, two, three and four
bytes and mixes them within one string.

**A codespace range is matched byte by byte.** §9.7.6.2:

> A code shall be considered to match the range if it is the same length as the bounding codes
> and the value of each of its bytes lies between the corresponding bytes of the lower and upper
> bounds.

That is not a comparison of the whole code against `low..=high`, and the difference is
load-bearing on the UTF-8-shaped `CMap`s four corpus documents write: `<C280> <DFBF>` admits
`C2 80` and not `C2 C0`. **No corpus document can tell the two readings apart** — replacing the
per-byte test with the numeric one leaves all 1794 oracle verdicts identical, which was checked
rather than assumed — so the only thing holding the clause to its words is a synthetic test.
That is trap 8 in one sentence: a corpus finds what documents contain, not what the
specification says.

### The identity `CMap` is one range, not 65 536 entries

Table 116 describes `Identity-H` as mapping "2-byte character codes ranging from 0 to 65,535 to
the same 2-byte CID value", and that is how it is stored. Every other predefined name is
refused and reported: 15 corpus fonts across 11 documents. Those names stand for registered
`CMap` **files**, so supporting them is a decision about vendoring third-party data and its
licence, not a coding one — and guessing at a mapping produces plausible text that says
something else, which is the failure trap 1 exists for.

### §9.7.6.3's recovery is implemented, and two references depart from it

An invalid code still shows a glyph, and the clause says how many bytes it consumes: the
codespace range with the longest partial match, ties going to the shortest codes, and the
shortest codes outright where not even the first byte matches. Then the notdef mappings, then
CID 0.

`issue11768_reduced.pdf` is where that becomes visible. Its `CMap` is `UniJIS-UTF8-H`, whose
codespace admits no one-byte code above `7F` — and it writes `1 begincidchar <e0> 151`, a
one-byte mapping for a code its own codespace excludes. Under §9.7.6.2 each `e0` is invalid, and
under §9.7.6.3 it consumes the three bytes its partial match against `<e08080> <efbfbf>`
implies. `mupdf` and `ghostscript` take the mapping's own length instead and draw three hyphens
where we and `poppler` draw one `.notdef`.

We follow the clause. The standard would not have written a two-rule recovery algorithm for
invalid codes if a mapping's length were meant to override the codespace, and principle 5's
direction of inference runs one way: their disagreement is a question for the clause, not a
target.

**A `notdefrange` maps its whole range to one CID**, where a `cidrange` numbers upward. ISO
32000-2 does not say so; §9.7.5.3 hands the file's syntax to Adobe Technical Note #5014, which
does, and §9.7.6.3's own purpose for the mapping — "to obtain a substitute character selector" —
implies it, because a run of undefined codes wants one substitute rather than a run of
consecutive CIDs the `CIDFont` has no reason to carry. This was wrong in the first draft and a
test caught it.

### `bfchar` in an `Encoding` `CMap` is read, because §9.7.6.2 names it

§9.7.5.4 c) is explicit:

> The beginbfchar and endbfchar shall not appear in a CMap that is used as the Encoding entry of
> a Type 0 font; however, they may appear in the definition of a ToUnicode CMap.

`bug920426.pdf` writes an `Encoding` `CMap` whose **only** character mappings are `bfchar` lines
with two-byte hex destinations. The clause forbids the file, and says nothing about what a
processor does with one — except that §9.7.6.2's own account of the decoding algorithm names
them among the character mappings:

> (These are the mappings defined by `beginbfchar` … and corresponding operators for ranges.)

So the two subclauses disagree, and the one describing what a processor *does* is followed: the
destination is read as the character selector §9.7.5.1 says a `CMap` yields, which for PDF is a
CID. The page draws "Checkliste Service" instead of nothing, which is what three reference
renderers draw. The same bytes read as `crate::tounicode` would read them — UTF-16BE text — are
a different clause's answer to a different question, and keeping the two modules apart is what
makes that statable.

### `/UseCMap` is followed for a stream and refused for a name

Table 118's `/UseCMap` supplies the map a file states only its differences from, its codespace
ranges included. A stream is parsed and built upon, with this file's own entries consulted
first; a predefined *name* is refused, because the mappings the referencing file inherits would
be missing an unknown share of their codes. §9.7.5.4 a)'s companion rule is enforced in the
other direction too: a `usecmap` operator with no `/UseCMap` entry beside it names a map that
cannot be found, so the font is refused rather than half-applied.

No corpus document exercises any of this. It is implemented with synthetic tests on trap 8's
argument, and the alternative — leaving a clause unread because no file in one collection
happens to use it — is what the conformance ledger exists to prevent.

### Glyph selection is decided by what the program *is*

`CidToGlyph` has three arms and they are §9.7.4.2's own cases: a CID-keyed CFF's charset, a
`/CIDToGIDMap` stream, and the identity. Which applies is decided by the embedded program rather
than by `/Subtype`, because a `CIDFontType0` may arrive as a CFF inside an `OpenType` wrapper and
the clause's two CFF cases are about what the Top DICT contains.

**An explicit `/CIDToGIDMap` stream outranks "the CIDs shall be used directly as GID values".**
This is the decision the page overturned. Table 115 conditions the entry's *presence* on Type 2
— "Required for Type 2 CIDFonts with embedded font programs" — and defines its meaning
unconditionally: "A specification of the mapping from CIDs to glyph indices." The first reading
written here took the presence condition as a restriction on meaning and ignored the stream for
a Type 0 `CIDFont`. `issue7901.pdf` is a `CIDFontType0` whose `/FontFile3` is an `OpenType`
wrapper around a *name*-keyed CFF, carrying a `/CIDToGIDMap` stream of 230 entries; under that
reading it drew `üãÍ†Ë œÍ†ÿ¨ Ì{«` where four renderers draw "The Free Software Definition". The
producer's CIDs are not that CFF's glyph indices and the stream is the only thing in the file
that says what they are, so a reading that discards it makes the file's own statement mean
nothing.

The order is therefore: a CID-keyed CFF's charset, which §9.7.4.2 states outright; then the
dictionary's stream; then the identity. A non-embedded program takes none of them — the clause
says the entry "shall be ignored, since it is not meaningful to refer to glyph indices in an
external font program" — and a substitute is reached through `/ToUnicode`, as before.

### Widths are indexed by CID, and word spacing by the code's length

`/W` is a table of CIDs (§9.7.4.3), so `advance` takes the code through the `CMap` first. A code
the `CMap` does not define takes CID 0's width, because CID 0's glyph is what §9.7.6.3 says is
drawn.

`LoadedFont::has_single_byte_codes` is gone. §9.3.3's word spacing is a property of the code —
"It shall not apply to occurrences of the byte value 32 in multiple-byte codes" — and answering
it per *font* was exact only because every mapping this crate built was wholly one-byte or
wholly two-byte. `Code::takes_word_spacing` answers it per code, which four corpus documents now
need.

## Consequences

**31 corpus documents draw with nothing reported that could not draw their text at all**, which
is the largest single movement since `CCITTFaxDecode`. The gate's `Text` row falls from 100
fonts to 67, and nothing left on it is an embedded-`CMap` or `/CIDToGIDMap` question: 27 fonts
have no `/ToUnicode` so a substitute cannot be addressed, 21 have a substitute that draws none
of their declared codes, 15 name a predefined `CMap`, 4 ask for vertical writing, and the rest
are malformed programs.

**32 pages joined the oracle's judged set and 18 of them agree outright.** Two are contradicted
and neither is a `CMap` defect:

- `issue7901.pdf` draws its sentence correctly and fails only the *differing-fraction* bound, at
  9.89% on a 200×40 page that is nothing but eight-pixel glyphs — every absolute bound is met
  with room. It joins `CONTRADICTED_GLYPHS_JUDGED_AS_VECTOR`.
- `issue20232.pdf` is missing one glyph, and the clause is §9.6.5.4 rather than §9.7: a simple
  `TrueType` font whose `/Differences` names code 71 `/Ccedilla` while its subset holds the
  diameter sign there, with a descriptor whose `/Flags` sets the Symbolic and Nonsymbolic bits at
  once. It is listed as `CONTRADICTED_SYMBOLIC_FONT_FLAGS` rather than chased, because changing
  which route a contradictory descriptor takes puts ADR 0015's fifteen pages at stake.

**Interpretation costs +0.44%**: 1.9259 G instructions to 1.9344 G by callgrind on
`examples/callgrind_interpret`, measured on this machine against a worktree at the previous
commit rather than against a number from an older session. That buys a codespace scan and two
map lookups per code where there had been a fixed two-byte chunk, and the corpus gate's wall
clock is unchanged at 1.9 s.

**§9.7's seventeen ledger rows are reviewed** — seven `implemented`, eight `partial`, one
`reported`, two `inapplicable` — and the two that stay short of complete are honest about why:
§9.7.5.2 needs data with a licence attached, and §9.7.4.3's `/W2` is §9.2.4's vertical-metrics
gap seen from clause 9.

## Alternatives considered

**Keep the Identity fast path as a separate mapping.** Rejected: `CMap::identity()` is one range
and one lookup, the general path is a codespace scan over a single entry, and the +0.44%
measured above is that cost on a page of 3587 glyph lookups. A second code path would be two
implementations of §9.7.6.2 and only one of them exercised by the documents that need it least.

**Expand a `cidrange` into entries at parse time.** Rejected for the same reason
`crate::tounicode` does not: `<0000> <FFFF>` is one line and sixty-five thousand entries, which
is a decoding bomb rather than a font.

**Be lenient about `issue11768_reduced.pdf`'s one-byte mapping under a UTF-8 codespace**, as
`mupdf` and `ghostscript` are. Rejected on principle 5: §9.7.6.3 prescribes what happens to a
code its codespace excludes, in two numbered rules, and inferring a code's length from a mapping
the codespace contradicts is a different algorithm from the one the clause describes.

**Read `/CIDToGIDMap` only for `CIDFontType2`.** This was implemented first, from Table 115's
"Required for Type 2", and `issue7901.pdf` rejected it. See above; the lesson is that a
presence condition is not a restriction on meaning.
