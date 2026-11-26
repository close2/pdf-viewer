# 3738 — An XFA template's unusable bound is returned to its reader, and an event fires on click

Session 2259, the silent-repair campaign's census burn-down, sixth pass. Status: **accepted**.
Context: ADR 3477, ADR 3574, ADR 3621, ADR 3709, ADR 3710; ISO 32000-2 §12.7.3; XFA 3.3 pages
597, 671 and 703.

## Context

ADR 3710 left `xfa/template.rs` with five sites: an `occur` `min` or `max`, and a `decimal`'s
`fracDigits` or `leadDigits`, that are no integer were read as the grammar's default in silence,
and an event's absent `activity` was read as no activity. It withdrew a marker on the last because
whether `click`, which the syntax lists first, is its default had not been read. The template
reader holds no document. Eleven more files were read beside it; most of their sites are bounds,
declines, writers or this program's own faces, and the rest are the repairs below.

## Decision

- **`activity` defaults to `click`.** Page 597's key to the syntax says "\[u]nderlined words
  indicate the default value. The application uses this value if the attribute is omitted", and
  on page 703 `click` is underlined. The markdown of `doc/md/` loses the underline, so the PDF's
  page was read: a 29.9-point rule under `click` and none under the other values. An event with
  no `activity` was never raised; now it is raised on a click. A defect, not a repair, and a test
  pins it.
- **`model/xfa-template-attribute-unusable`** (§12.7.3's `/XFA`): `Template::read_noting` returns
  the class, ADR 3621's shape; `xfa.rs` records it at the template's packet stream. A `fracDigits`
  or `leadDigits` below `-1` is unusable too: page 671 gives `-1` a meaning and no other negative.
  Mapped to section 5.1 alone; the converter's answer is ADR 3521's whole-file rewrite, which drops
  `/XFA` as the archive profiles require anyway.
- **Recorded on existing classes**: a choice field's `/I` holding no usable index and a widget's
  `/Parent` that is no dictionary (`field-entry-unusable`); an annotation's `/P` naming no page
  (`annotation-entry-unusable`), now read as absent so the `/Annots` walk finds the page, where it
  had answered page one.
- **Marked** with the reason each is no repair: the public-key handler's `/V` (logged by
  `Document::authenticate`), its `/CF` lookups (declines, or the identity filter's empty list);
  bounds of `unpack.rs`, `bidi.rs` and `joining.rs` (UAX #9 and `DerivedJoiningType.txt`'s
  `@missing` value); the writers `type1_to_cff.rs`; the finder `cff_reals.rs`'s declines; and
  `stretch_cff.rs` and `cff_stretch.rs`, which rewrite a face this program ships, held to the
  read-back proof.

## Consequences

- `HELD` 287 → 224 (`xfa/template.rs`, `submission.rs`, `image/unpack.rs`, `stretch_cff.rs`,
  `bidi.rs`, `cff_reals.rs`, `type1_to_cff.rs`, `joining.rs`, `cff_stretch.rs`, `public_key.rs`
  to 0; `xmp.rs` 4 → 2; `script_sites.rs` 4 → 1).
- Held as repairs: `xmp.rs`'s numeric character reference that does not parse, kept verbatim by
  `unescape` (fourteen callers, no document), and `script_sites.rs`'s page `/Annots` that is no
  array, which no reader of `/Annots` logs yet and no class names.
