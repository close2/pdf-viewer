# 1193 — A value that crosses from another file is copied

The forms round of batch 1189–1194. Rows: §12.7.8.3.2 (`partial`, three of four entries built and
the fourth narrowed), §12.7.8.3.4 (`partial` → `departed`), §12.7.6.2 (`partial`, note made exact),
§12.5.6.2 (`partial` → `implemented`).

## The decision (ADR 1223)

ADR 1186 left four Table 249 entries unapplied for one reason: their values are indirect references
into the FDF file. The obvious answer — a second `Document` in the interpreter — is the wrong one,
because the oracle's comparison rests on `interpret` being a function of *the bytes*. So nothing
crosses as a reference: `forms_data::carry` resolves every reference and copies its value in place,
bounded, refusing a whole entry rather than half of it. `/AP` replaces the widget's appearance
dictionary; `/A` and `/AA` compose over the widget's own, so Table 197's precedence is read once;
a save writes the imported `/AP` into §7.5.6's update — the FDF producer's own marks, which is
`CLAUDE.md`'s provenance test rather than a new permission.

## The two readings (ADR 1224)

**§12.7.8.3.4 needs no second standard.** The clause is one sentence about Table 254's `/Page`;
everything else in such a dictionary is §12.5's, because an FDF annotation *is* an annotation. ISO
19444-1 defines XFDF, not this — which is what the row assumed, and `Q97` now asks for that text.

**`/DS` is not an entry of Table 172.** It appears in §12.5.6.2 once, inside the group-attribute
list, and is defined in Table 177. What this clause owes for it is the group rule, already applied
— and `appearance::unapplied_default_style` now reports the XFA departure on the note it would have
styled. `/Subj` and `/CreationDate` reach `popup::Popup` and `viewer_core::PopupWindow`.

## Found, not built, and the gates

`/APRef` was recorded as a host question. Table 253 makes `/F` optional and says what its absence
means: the page is in the associated PDF file — two branches, one needing a filesystem and one
reaching §12.7.7's name tree this crate already reads. On the row, unbuilt.

`rustfmt --check` on twelve files: 0. `clippy -p pdf-model --all-targets` under `-D warnings`:
clean of this round's work (two neighbours' errors stood). `clippy -p viewer-core`: 0. `nextest -p
pdf-model` 1519 passed, `-p viewer-core` 231 passed, `test -p conformance` 275 passed. Behind the
lock: `pdf-model --test corpus` 0, `raster_golden` 0 with **0 moved**, `save_round_trip` 0. Trap 1:
no corpus page moves; the new marks are exercised by the text readback the drawn glyphs produce.
`Q97` (ISO 19444-1 sections 6.4/6.6) and `Q98` (the submit-form network) are the owner's.
