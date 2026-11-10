# 2795 — What the first pass wrote unjudged is converted again, and a `DeviceCMYK` group at part 2 takes the inks

Session 1992, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/mod.rs`: the second pass's condition; `archive/settle.rs`;
`archive/report.rs`: the `settled` sentence; `archive/prepare.rs`: the stand-in press at part 2;
`archive/supplied_cmyk.rs`: `blends_in_cmyk`, `prepare`'s `groups_only`; `archive/decision/colour_and_graphics.rs`:
`rgb_answer`, `cmyk_answer_under_part_two`).
It answers four lines of `doc/pdfa-blocking-issues.md` section 8 (`D-defect`).
Context: ADRs 2114, 2130, 2518, 2534, 2747.
Clauses: ISO 19005-2 sections 6.1.7.2, 6.2.4.3, 6.2.10, 6.2.11.6; ISO 19005-4 sections 6.1.6.2, 6.2.9, 6.2.10.6;
ISO 32000-2 §7.5.5 (Table 15), §8.6.5.6, §9.6.5.4, §11.6.6 (Table 145).

## 1. The causes, traced

- **`bug864847` (2b).** The font is a symbolic TrueType (descriptor `/Flags 4`) with no program. ISO 19005-2 section
  6.2.11.6's `cmap` paragraph reads an embedded program, so the source meets it; the face this program substitutes
  carries (0, 3), (1, 0) and (3, 1), which part 2 refuses (one subtable, or (3, 0) among them) and part 4 admits
  ((1, 0) present). The rewrite that answers the row (`TrueTypeCmapRestated`) exists and was never asked, because the
  row failed only in the output.
- **`JA1-1996-PDF`, `JA3-1998-PDF` (both parts).** The first-page trailer of each linearised file states
  `/Prev 576066 0 R`, a reference where ISO 32000-2 Table 15 asks for a byte offset, so the main section is never
  read. The reader finds every other object by its own header (`declared_and_unread`, ADR 0789), and the check
  enumerates `XrefTable::object_numbers`, which is the first-page section's ten. The thumbnails' `LZWDecode` streams are
  therefore never judged in the source, and the output, whose fresh table lists every object, fails section 6.1.7.2.
- **`auth-event-ef-open` (4).** The embedded file is encrypted under a crypt filter whose `/AuthEvent` is `/EFOpen`
  and which this reader cannot decrypt without the attachment's password (`attachments --save-all` refuses it
  `DecryptionFailed`). The source fails `crypt-filter-is-identity` and meets the embedded-file row, which cannot read
  the file; the first pass drops the encryption and copies the stream with its `/Filter [/Crypt]`, so the output fails
  both rows.
- **`issue14200`, `issue14297`.** A form's transparency group states `/CS /DeviceCMYK` and the document's intent is
  sRGB. The group rows are answered by `RgbUnderPartTwoOrFour(None)`, which took the stand-in press only where an
  operator supplied one, so the output intent was decided and the output failed. At part 2 no stand-in was prepared at
  all: the `DeviceN` default is part 2's answer for a mark, and ISO 32000-2 §11.6.6's Table 145 forbids `DeviceN` as a
  group's colour space while remapping a device one through the current resource dictionary's defaults.

## 2. The decision

1. **The second pass (ADR 2534) also runs where the first output fails a requirement the source met** (its
   `regressions`), and not only where appearances were constructed. A regression is by definition content no pass over
   the source judged: a substituted program, an object the source's check never enumerated. The pass is the same one
   pass, under the same plan, with the same report (`settled`) and history; the sentence now says "that output held
   what no pass over the source judged", which is true of both triggers.
2. **A family-less colour row** (a group's `CS`, a base, an alternate) **takes the stand-in press as part 4's
   `DeviceCMYK` row would**, where one was prepared: the assumed inks' profile as a statement (ADR 2747), the shipped
   press as `press-assumed` where that loss is authorised (ADR 2518), the configured press likewise.
3. **At part 2 the stand-in is prepared where the group row failed, and is built only for a group blending in
   `DeviceCMYK` with no `/DefaultCMYK`** (`blends_in_cmyk`). A `DeviceCMYK` mark beside no such group keeps the
   `DeviceN` default, which states no press; where both exist, the one ICC default answers both, because a `DeviceN`
   default in that dictionary would remap the group into a space Table 145 forbids. This revises ADR 2518's "part 2 is not offered it"
   for a group, whose argument (the `DeviceN` default states no press) has no default to offer there.

Each profile: `as-if-printed` authorises `press-assumed`, so its group default is the shipped press, named as that loss
in the report and the history (ADR 2693's rule); every other profile takes the inks' profile, a statement of what the
window drew. `no-data-loss` keeps refusing `auth-event-ef-open` at 4: the attachment's removal is a loss it does not
grant.

## 3. What it costs

- A refused document that regressed is converted twice; one that is written was never refused, so it pays nothing. On
  the seven freed documents the verb's wall time is under a second each.
- At part 2, a document whose group blends in `DeviceCMYK` and that also paints `DeviceCMYK` marks now has its marks
  read through the inks' profile rather than through the `DeviceN` default; the report says so.

## 4. Not built

- **The check's enumeration.** `Examination::objects` should include the objects a cut chain leaves to their headers;
  `pdf_syntax` exposes the header scan to no other crate, and the second pass already makes the converter's output
  right. A source claiming PDF/A with such an object is still judged on the first section alone.
- **`/Crypt` on a stream whose encryption is dropped.** The first pass still writes it; the second pass removes the
  attachment it guards under the profile's embedded-file answer, which is what a correct first pass would have done
  with an attachment it cannot read.
- **`issue14200` at 2b.** Its soft mask's group form (`/G`) states no `/Resources`, and `TechNote 0010` A028 reads part
  2's default only from the explicitly associated dictionary; `DefaultSites` writes no dictionary into a form that has
  none, and giving it one means copying the dictionary §7.8.3 lends it.

## 5. Evidence

The freed outputs and their pixels are in `doc/history/1992-*.md`. Tests, in `pdf-transform/tests/archive.rs`:

- `a_face_substituted_for_a_symbolic_truetype_font_is_converted_as_the_program_it_is` and
  `an_object_found_only_by_its_header_is_converted_as_the_content_it_is` fail with the condition restored;
- `a_group_blending_in_device_cmyk_beside_an_rgb_intent_takes_the_assumed_inks` fails without the part 2 stand-in;
- `a_device_cmyk_mark_at_part_two_keeps_the_device_n_default_beside_no_cmyk_group` fails with the stand-in built for
  marks.
