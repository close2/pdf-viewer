# 1700 — `getAnnots` answers the document's markup, and a script writes what a reader's edit reaches

Status: accepted and **built**. Session 1432. Builds on ADR 1689's ranking (`this.getAnnots` first,
34 documents in the static census), ADR 1626's `getOCGs` (an object of the realm's own whose write
is a reader's edit) and RFC 0008 section 4.2's `Doc` row.
Code: `crates/pdf-model/src/view/script_annotations.rs` (the reading, the overlay, the change and
the save), `crates/pdf-model/src/view/script_model.rs` (`AnnotationState`, `AnnotationChange`,
`ScriptEdit::Annotation`, `DocumentState::annotations`), `crates/pdf-model/src/view/scripts.rs`
(`Told::annotations`, the edit applied, `this.dirty`), `crates/pdf-model/src/view/script_sites.rs`
(`page_of`), `crates/pdf-model/src/view.rs` (the save's call), `crates/pdf-model/src/popup.rs` (a
script's `popupOpen` on the drawn window), `crates/pdf-script/src/engine/annotations.rs` (the three
calls and the `Annotation` object), `crates/pdf-script/src/wire.rs` (version 8, shared with ADR 1702).
Tests: `crates/pdf-script/tests/annotations.rs`, `crates/pdf-model/tests/script_annotations.rs`,
`crates/pdf-script/tests/wire.rs`.

## 1. What a script asks, and where each answer is read

Adobe's *JavaScript for Acrobat API Reference* at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7` —
"Doc methods" (`getAnnot`, `getAnnots`, `syncAnnotScan`), "Annotation", "Annotation properties" and
"Annotation types" — is cited and never quoted; every rule below is a documented choice under
principle 5. Each property is read from the entry ISO 32000-2 states for it: `type` Table 166's
`/Subtype`, `page` the page whose `/Annots` lists it, `rect` `/Rect` normalised as §7.9.5 reads a
rectangle, `name` `/NM`, `contents` `/Contents` (or a person's retyping), `author` Table 172's `/T`,
`modDate` `/M` as §7.9.4's date, `hidden` and `readOnly` Table 167's bits 2 and 7 as the reader sees
them, `popupOpen` Table 186's `/Open` read as `crate::popup` opens the window, or on a text annotation
with no popup Table 175's `/Open`.

## 2. The choices

- **The annotations are the seventeen subtypes "Annotation types" lists.** A widget is a `Field`, a
  link and a popup are not on the list, and the popup is reached as its parent's `popupOpen`.
- **`getAnnots` answers in page order, each page in its `/Annots` order with a person's additions
  after** — §12.5.2's drawing order — for the reference's default `ANSB_None`; `ANSB_Page` is the same
  order and the other three sorts are stable over it. `null` where nothing is found, as the reference
  says. Arguments are positional or one object of named properties, as `app.alert`'s are.
- **The constants are numbered from zero in the reference's order**, which names them and publishes
  no number. **Only `ANFB_ShouldNone` filters**; the other six are Acrobat's panel, summary and export
  rules, named and not defined, and each is refused by name.
- **`getAnnot(nPage, cName)` finds `/NM` on that page**, the first in page order where two share a
  name. **`syncAnnotScan()` does nothing**: a realm is told every page's annotations before its first
  script runs, which is what the reference's background scan exists to reach.
- **Three writes, the three a reader's own edit reaches** (RFC 0008 section 4.2): `hidden` is
  §12.6.4.11's hide — the same override set a hide action keeps; `popupOpen` is what a click does to
  the window, held beside the edit log and drawn by `crate::popup`; `contents` is a person's retyping,
  which this program's edit reaches on a free text annotation alone. Every other write, and
  `contents` on another subtype, and `popupOpen` on an annotation with no window, is refused by name.
- **A save writes each as §7.5.6's update of that annotation**: Table 167's bit 2 into `/F` (the bit
  the reader sees, so a later hide action wins), Table 186's or Table 175's `/Open` — and a popup
  closed under a text annotation whose own `/Open` is true clears that entry too, since either opens
  the window — and `/Contents` with its regenerated appearance through the retyping's own writer.
  `this.dirty` counts them. Table 167's `ReadOnly` is about the user's interaction and is told, not
  consulted, as a hide action does not consult it.
- **An annotation event's `this.pageNum` is the page whose `/Annots` lists it where `/P` names none**
  (`page_of`). Table 166 makes `/P` Optional; the LaTeX `cooltooltips` documents
  (`poppler-11865-0.pdf` and its two siblings) omit it and call `this.getAnnot(this.pageNum, …)`,
  which answered `null` on page one and threw.

## 3. The cost, and why it is not on the launch path

The reading is made once per realm, at its first telling, and only where a runner is supplied and a
script runs; later tellings lay the view state's changes over it. Measured with a timing patch
applied and reversed, over the Tier 1 column's population: 149 realms read, median 62 µs, mean
182 µs, largest 5.86 ms (755 pages, no markup); 18 of the 149 documents hold any scripted annotation,
the most 24. ADR 1653 section 4's 77 MiB walk was a field table built on every document; this walk is
a scripted document's alone. A realm is told at most 32 768 annotations, half the wire's count, and
a document past it is reported.

## 4. What the census population shows

`grep -l getAnnot` over the population's 90 763 files names 27 (the census's 34 also reads scripts in compressed streams, which a byte grep cannot). 17
carry the code of Adobe's attachments shim, 16 under its name `ADBE::FileAttachmentsCompatibility`,
which reaches `getAnnots` only where `app.viewerVersion` is below 7 and so skips it here; 3 are
`cooltooltips` documents, whose seven annotation events on page three now run with no report (each
`getAnnot` found, its `popupOpen` written); 4 call `getAnnotRichMedia` and 2 `getAnnots3D`, clause
13's exclusion; `PDFIUM-1198-0.pdf` calls `getAnnots` with four absurd numbers from no site the open sequence
runs.
No run of the Tier 1 column reaches either member, since it raises no annotation event.

## 5. Unfinished

- A text annotation with no popup has no window drawn by `crate::popup`, so its `popupOpen` is
  saved and told and not seen until a later reader opens it.
- A person's later click on a window a script opened is `viewer-core`'s own map and wins; a script's
  later write does not reach past it.
- `contents` on the other sixteen subtypes waits on a reader's edit of `/Contents` for them.
- The six filters, and the other Annotation properties and methods (`setProps`, `destroy`, …).
