# 2504 — A font naming a face with no widths converts behind the loss `layout-from-this-viewer`

Status: implemented, in `pdf-transform` (`archive::decision`, `archive::fonts::described`,
`archive::prepare`, `archive::report`). Session 1835, the archive slot of batch one hundred and
twenty-four. It supersedes ADR 2477, which deferred the build. Context: `doc/questions/Q687` and
`doc/questions/A687` (the owner's word, option (b)), ADR 2146 (the standard 14 by name alone, whose
`describe` this opens), ADR 2476 (a renamed descriptor keeps a copy of the producer's own), ADR 2490
(the blocks reopen behind the remedy profiles) and `doc/questions/A48`, whose rule against filling in what the file never stated
this build departs from on purpose: the widths are the filling-in of an absence A48 names, which A687
takes deliberately and only behind the named loss, so that no reader is told they are the producer's.
ADR 2505 records the supplied name and what stays refused.

## What the owner decided

Q687 asked whether `described::describe` may be opened to a simple font outside the standard 14 that
states neither `/FontDescriptor` nor `/Widths`, a refusal that held ten documents of the pdf.js and
`doc/corpora` surveys, alone in each. Option (a) was to wait. A687 chose (b): lift the refusal behind a
named loss that `as-if-printed` discards and `as-shown` stops on, and supply a conforming
`/BaseFont` where the font names none so that veraPDF's 6.2.11.2 test 3 passes.

## What converts

ISO 32000-2 §9.6.2.1's Table 109 makes `/FirstChar`, `/LastChar`, `/Widths` and `/FontDescriptor`
required of every simple font but the fourteen, and Annex D publishes those fourteen's metrics; for any
other name the standard states no layout. The viewer draws such a font all the same, from the shipped
face whose metrics its name resembles, at the advances `pdf_font::LoadedFont::advance` reads. The
conversion embeds that face and states those advances as the font's `/Widths`: **the proof is the
one ADR 2146 built, unchanged** (`described::proved` writes the converted font into a document of its
own, loads it through the viewer's reader and compares it with the source font code by code, glyph and
advance, to a thousandth of an em). What changes is only who chose the layout: the page is the viewer's
and another reader may have set the same name at the advances of the face the name denotes. That is the
loss.

- **`Loss::LayoutFromThisViewer`**, word `layout-from-this-viewer`, in `Loss::ALL`, with the sentence
  `Loss::describe` gives it and its field in `Authorisations`.
- **It is conditional at the font site**, in the same set as `machine-face` and `substitute-stretch`
  (`CONDITIONAL` for `fonts/font-programs-embedded`): the profile's answer to the site governs. So
  `as-if-printed`, whose answer is `discard`, authorises it, and `as-shown`, whose answer is `stop`,
  does not; neither profile file gains a key. The tests name both profiles.
- **`describe` refuses by name in three cases** (`layout_of` in `described.rs`), each a sentence a person
  can act on: a subtype that is none of `Type1`, `MMType1` and `TrueType` (the standard says nothing of
  what a font of another subtype is drawn by); a font that states `/Widths` and no descriptor (its
  layout is the file's own and the face embedded is not the one the widths were stated for); and a
  `/BaseFont` present but not a name (no name for `/FontName` to equal, and replacing the entry is not an
  interpretation the standard defines).
- **The descriptor's `/FontName` is the file's `/BaseFont`, byte for byte, where the file states one.**
  Table 120's `/FontName` row says the value "shall be the same as the value of BaseFont in the font or
  CIDFont dictionary that refers to this font descriptor", so the name the producer wrote is the one the
  descriptor takes, whether or not it resembles the embedded face's name; a name the file does not state
  is ADR 2505's.
- **The font dictionary is restated as ADR 2146 does**: the subtype is rewritten when the face's format
  needs another (a `TrueType` or `MMType1` source given a Compact Font Format face becomes `Type1`, with
  an encoding restated where the source had none, ISO 32000-2 §9.6.5.1), and a stated `/Encoding` is kept.
- **The loss is named in both places a loss is named.** The report's font line says the file states no
  widths for the font (the route sentence "the face's own advances are the widths the file states" would
  be false of it), and the `xmpMM:History` entry carries the clause "(the file states no widths for this
  font; the widths written are the layout this viewer drew it with)" beside the loss word, which `prepare`
  builds from the same `SubstitutedFont`.

## Evidence

The census is `the_shipped_profiles_list_what_real_producers_files_refuse_on`, in an export of HEAD and
an export of HEAD with these hunks, with `QUORRA_CJK_PACK` naming a copy of the owner's pack (its five
programs held to their digests), both shipped profiles at 2b and part 4, pdf.js and `doc/corpora`
(converted / conforming / refused / unreadable). `as-if-printed` 2b: 804/2/149/11 → 809/2/144/11 and
351/6/116/16 → 354/6/113/16; part 4: 807/0/148/11 → 812/0/143/11 and 353/0/121/16 → 356/0/118/16.
`as-shown` is unchanged at all four, as its `stop` requires. The eight documents are `issue6069`,
`issue6106`, `issue6108`, `issue18099_reduced`, `issue2391-2`, `All Quiet on the Third Coast` and the
hand-built `T02-04-01_001_resources-basefont-missing` and `T02-04-01_002_resources-basefont-wrong-font`;
the other two of A687's ten stay refused (ADR 2505). veraPDF 1.30.3 passes all sixteen outputs, the eight
at each target, the one with no `/BaseFont` included, and every output re-renders through the viewer.
Round 1821's experiment passed fourteen of sixteen outputs; the two it failed were the hand-built
document with no `/BaseFont`, at both targets, which ADR 2505's name answers.

`cargo tree -i pdf-transform --edges all` reaches no render crate, so against HEAD's six arms no page's
digest can move; worked in `scratchpad/r1835/`.

## The tests

`tests/archive/layout_from_this_viewer.rs`: the three subtypes convert at both parts with the loss, the
stated `/BaseFont`, the `/FontName` and a `/Widths` array written; the widths are the layout the viewer
drew, to a thousandth of an em, read back through the viewer's own reader; a stated `/BaseFont` is kept
byte for byte (a UTF-8 name, because `file-structure/bound-names-are-valid-utf8` refuses a name of other
bytes earlier); the standard 14 by name do not incur the loss; without the authorisation nothing is
written and the loss is named; the three refusals; the two profiles; the history clause; the report
sentence. Planting the defect each guards: with the `/BaseFont` write disabled, the no-`/BaseFont` test
fails; with the loss left unrecorded five tests fail.

## What this does not decide

A reader that held the face the name denotes would draw the document at that face's advances; the
output states the viewer's. This is the loss, said in the report, in the file's history and by
`as-shown`'s refusal. Nothing here chooses a font the file did not name: the face is the one the viewer
already drew.
