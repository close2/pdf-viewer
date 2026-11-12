# 2846 — A font name that is not UTF-8 goes with the program the conversion embeds

Session 2014, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/fonts/names.rs`, the branch at the head
of `archive/decision.rs`'s `decide`, `UTF8_NAMES`'s sentence).
Context: ISO 19005-2 section 6.1.8, ISO 19005-4 section 6.1.7; ISO 32000-2 Tables 117, 119 and
120, §9.9.2; ADRs 2302, 2807, 2838; R03 of `doc/pdfa-blocking-issues.md` (rank.py files
`file-structure/bound-names-are-valid-utf8` under R03-photograph).

## 1. What the census said

Six documents stop at `file-structure/bound-names-are-valid-utf8` under every profile at 2b and 4,
and three of them (`issue2128r`, `issue3521`, `issue5010`) at nothing else under `as-if-printed`.
Every place is a font name written in a Chinese or Korean code page (`BA DA CC E5`, GBK's bytes for
a two-character face name), on a Type 0 font, its descendant and the descriptor; no colourant or
structure type name is among them. The row was a fence: "renaming one would break every reference
that uses it".

## 2. The decision

That sentence is true of a colourant name and a structure type name, which other objects find the
thing by, and of a font whose program the file embeds, whose name Table 117 makes the program's own.
It is not true of a font this conversion embeds a program for. The pack, the machine-face and the
re-keying routes already write the descendant's `/BaseFont` and the descriptor's `/FontName` as the
embedded program's name (Tables 117 and 120); the producer's name was a request for a face, which
the report and `xmpMM:History` keep as the font's `requested` name. What was left is the Type 0
font above it, and Table 119 gives that name a value of its own — the descendant's name, a hyphen
and the CMap name for a `CIDFontType0`, the descendant's name for a `CIDFontType2` — with a NOTE
calling it "an arbitrary name, since there is no font program associated directly with a Type 0
font dictionary". `names::restate` writes that name into the Type 0 font's gains once every route
has run, and the row is answered where every place it failed at is a font or descriptor whose
font names the output writes as UTF-8: by the embedding, at the embedding's own losses
(`Substitutes::losses`), so a profile that refuses the face refuses the name with it.

A place the output no longer reaches answers itself: a descriptor a route replaced with one of
its own, and a font only a photographed page's resources named, are in no object the rewriter
writes from the catalog (`names::reached`, the walk ADR 2454's `still_held` makes for the report).
So where `as-if-printed` and `state-every-loss` photograph the page for the font (ADR 2516), the
row goes with the photograph at its losses, `text-as-image` and `page-as-image`.

No profile's choice moves: the row is answered under every profile that authorises the face or
the photograph, and the losses are theirs. Nothing is lost that the face did not
already lose; the Type 0 name is no font's name and nothing reads it (the ledger's §9.7.6.1 note:
this tree's reader leaves it unread).

## 3. What stays refused, and why

A colourant name, a structure type name, a font name written inside another object, a font no
page shows and so no route embeds (`issue3521`'s bold, italic and bold italic fonts: with the CJK
pack its shown font is embedded and its page is not photographed, so the three keep the row under
every profile; without the pack the page is photographed under the wide gate and they go with it), and a font whose program the file embeds (`issue5010`:
a CFF program under `/FontFile3` whose own name is the EUC-KR one) — renaming the last means
rewriting the program's Name INDEX, a font-program edit no round has argued. `SimFang-variant`
and `XiaoBiaoSong` are simple TrueType fonts whose stated `/BaseFont` the simple route keeps (ADR
2504), and stay here under every profile; `issue20489` moves to R11's
`fonts/symbolic-truetype-states-no-encoding` under `as-if-printed` and `state-every-loss`.

## 4. Not chosen

Transcoding the name from the code page its CMap implies (GBK for `GBK-EUC-H`) would guess an
encoding the standard does not state for a name, and keep a name no program carries.
Photographing the pages that use the font would take the text of every page that shows it for a
name nobody reads.
