# Specification audit of ISO 32000-2 §8.9 to §8.11 (images, form XObjects, optional content) — round 2195

The project's twelfth specification audit, made beside the other lanes in the shape of the
owner's review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits
2128 to 2184 ([`2184-specification-audit-8-6-8-7.md`](2184-specification-audit-8-6-8-7.md)).
Baseline: main at `8c0bcf0e`. Evidence:
[`2195-specification-audit-8-9-8-11/`](2195-specification-audit-8-9-8-11/) — `findings.json`
(2038's schema) and `expected-failures.json` (the failing tests of each behaviour finding). The
same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §8.9, §8.10 and §8.11 (Tables 87 to 101) was read in
   `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 6259 to 7145, and set against the code that reads
   the entry or performs the step. Most are constraints on the file; those were read for what a
   reader does when a file breaks them, and only a silent wrong result was taken further.
2. 2038's findings on these clauses were read first and are not repeated: B11, B14, B32, B51,
   B73, D06, D19 and its T findings (T45, T66, T92, T94, T101, T123, T147, T150, T152).
3. The readers traced: `pdf-model`'s `image/` (`dispatch.rs`'s bit depth per filter, `mask.rs`'s
   colour key and explicit mask, `soft_mask.rs`), `inline_image.rs` (Tables 91 and 92, the
   entries "ignored"), `content/image.rs` (§8.9.5.4's selection, under Errata Collection 3 as its
   comment states), `content/xobject.rs` (§8.10.1's steps, a reference `XObject`'s import and
   `enter_imported`), `content/annotations.rs`, `content/run/marked_ops.rs` and `marked.rs`
   (`BDC /OC`), and `optional_content.rs` whole (Table 97's two routes, Table 99, the usage
   application dictionaries and `reapply`) with `view/optional_content.rs` and the layer panel's
   `viewer/answers.rs`.
4. The lens was the earlier audits' habits: a rule keyed on a name applies wherever the name is
   read (Table 93's `/OC` is read on a `Do` and on an annotation, and an appearance stream is a
   form XObject too); list every route to a thing (Table 97's `/P` and `/VE`; an optional content
   state reached by the page and by an imported page); an absent entry is a third answer (a group
   with "no effect on visibility" is neither ON nor OFF).
5. About sixty fixtures were probed through `pdf_model::interpret` and the CPU oracle, and each
   B finding has a failing `*_spec_audit` test whose controls pass on main: `pdf-model/tests/xobjects_oc_8_9_8_11_spec_audit.rs`, a module of
   `content_suite`.

What holds, probed or traced, is not listed one by one. Among it: a colour key compared before
`/Decode` (inverted `/Decode`, `Indexed`, 4- and 16-bit samples, a CCITT image's 1-bit samples),
and a key of the wrong length reported; an explicit mask's `/Decode [1 0]`, and a `/Mask` without
`/ImageMask` reported; `/SMask` overriding a colour key, an explicit mask and the graphics state's
soft mask while `ca` and the blend mode stay; a soft mask's own `/SMask` ignored and reported;
`/Decode` results clamped (an `Indexed` index on both sides, `DeviceGray` above 1); `Lab`'s default
`/Decode`; a stencil's `/Decode [1 0]` inline; row padding; an inline image's `/Mask` ignored,
`G`/`RGB` never a resource, `/I` with an abbreviated base, a named `Indexed` resource, and `/L`
read past an `EI` inside the data; a form's `/BBox` clipped in form space through `/Matrix`, a
reversed `/BBox`, the state restored after `Do`, a `/Group` of unknown `/S` drawn as an ordinary
form; hidden content applying colour, `cm`, clipping and the text position and still drawing
nothing inside a form, a tiling cell, a Type 3 glyph or a soft-mask group; a `BDC` left open by a
form closed at its end, and a `BDC`…`EMC` split over a `/Contents` array; `/P`'s four policies,
`/VE` preferred to `/P`, null `/OCGs` entries ignored, `/BaseState /Unchanged`, no
`/OCProperties` ignoring every structure, a configuration `/Intent []` showing everything.

Readings left, each a question about the standard rather than a defect found: Table 87 says a
`CCITTFaxDecode` or `JBIG2Decode` filter "shall always deliver 1-bit samples" and §8.9.5.1 that an
inconsistent dictionary "shall cause an error", and an image stating `/BitsPerComponent 8` over
CCITT is drawn at the filter's depth without a word (lane 2's silent-repair inventory, round
2194, is where that belongs); the same table says `RunLengthDecode` "shall always deliver 8-bit
samples", which every 1-bit run-length mask in the world contradicts; a soft-mask group's own
Table 93 `/OC` has no `Do` to be skipped "as if there were no", and is not read; a `/FormType`
other than 1 is drawn as type 1.

## Findings

Three behaviour gaps. In order of how much a person would see:

- **2195-B03** — an annotation's appearance stream whose own form dictionary states `/OC` naming
  an OFF group is drawn. §12.5.5 makes every appearance stream a form XObject, and Table 93's
  `/OC` skips an invisible form entirely; the reader asks `/OC` at a `Do` and on the annotation
  dictionary, never on the `/AP` stream it runs itself.
- **2195-B01** — a visibility expression counts a group that takes no part (outside the
  configuration's `/Intent`, or never listed in `/OCGs`) as ON, so `[/Or off design]` shows what
  `/OCGs [off design] /P /AnyOn` hides and `[/Not design]` hides what `/OCGs [design] /P /AllOff`
  shows: §8.11.2.3's "no effect on visibility" is honoured on one of Table 97's two routes.
- **2195-B02** — an imported page's optional content is the target's `View` state at
  magnification 1.0 whatever the operation: `enter_imported` carries the purpose and the
  magnification onto a `ViewState` and then reads the groups afresh beside it, so a target layer
  its own `/AS /Print` hides is printed when the page importing it is printed (and `/Zoom`,
  `Language` and `User` are not followed either).

Two documentation findings: D01, `operand`'s comment says a non-participating group's `true` "is
the same thing as it having no effect on the result", and `evaluate`'s that a two-operand `/Not`
"evaluates to visible" — the first is B01's cause, the second false when the first operand is ON;
D02, a `[/Not]` with no operand is reported as "a /VE visibility expression nested past the
interpreter's bound", the depth bound's sentence for a one-level malformed expression.

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `8.11.2.3` (B01: "a
group whose intents miss the configuration's has no effect on visibility"), `8.11.2.2` (B01, D01,
D02), `8.11.4.5` (B02), `8.10.4.1` (B02), `8.10.2` (B03: "Table 93's entries that decide what is
drawn are read"), `8.11.3.3` (B03: "/OC on a form or image XObject"). `12.5.5` is `departed`, and
its note is read by the fix round for B03 as well.

## Suggested fix groups

1. **`pdf-model/src/optional_content.rs`'s `evaluate` and `operand`** — B01, D01 and D02: a
   non-participating operand drops out of its operator, an expression with nothing left leaves the
   content visible (the `/P` route's empty-states rule), a malformed `/Not` answered as the other
   malformed expressions are, and both comments restated.
2. **`pdf-model/src/content/xobject.rs`'s `enter_imported`** — B02: the imported page's optional
   content taken from the `ViewState` built there, and the audience carried with the other
   reader-side factors.
3. **`pdf-model/src/content/annotations.rs`'s `draw_appearance`** — B03: the appearance stream's
   own `/OC` asked beside the annotation's.
