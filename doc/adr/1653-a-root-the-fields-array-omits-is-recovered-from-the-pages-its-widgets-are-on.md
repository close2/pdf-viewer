# 1653 — A root the `/Fields` array omits is recovered from the pages its widgets are on

Status: accepted and **built**. Session 1408. Builds on ADR 0736 (the other recovery: a `/Fields`
entry stating a `/Parent`) and ADR 0245 (a widget the field tree does not reach is drawn and handed
to nobody); answers `doc/todo/56`'s Tier 1 column class 5.
Code: `crates/pdf-model/src/view.rs` (`widgets_by_field_name`, `widgets_on_page_by_field_name`,
`Omitted`, `field_table`, `field_table_on_page`, `recover_omitted`, `omitted_root`, `named_root`,
`field_at`), `crates/pdf-model/src/form.rs` (`fields`), `crates/pdf-model/src/view/scripts.rs`
(`recalculate_with_runner`, `displayed_values`, `apply_resumed`, `Told::names`),
`crates/pdf-model/src/view/script_sites.rs` (`run_open_scripts`, `run_page_scripts`).
Instrument: `crates/pdf-model/examples/orphan_field_census.rs`.
Tests: `crates/pdf-model/src/form.rs`
(`a_root_the_fields_array_omits_is_recovered_from_the_page_and_a_nameless_chain_is_not`, and its
control `a_listed_root_and_an_omitted_one_read_alike`).

## 1. What the file says, three times

ISO 32000-2 §12.7.3's Table 224 defines `/Fields` as "(Required) An array of references to the
document's root fields (those with no ancestors in the field hierarchy)". Table 226's `/Parent` is a
field's own statement of the field above it, and §12.7.4.2 makes a dictionary stating a `/T` a field
with a name. A widget a page's `/Annots` lists whose `/Parent` chain climbs to a dictionary stating
a `/T`, where no `/Fields` entry reaches that dictionary, is a field the file states twice and omits
once. The tree read it as the omission said: drawn, its appearance and `/V` intact, and no field —
no control for a host, no value a person could type, no name a script's `getField` or a hide action
could reach.

## 2. The choice: recover the root, walked as a `/Fields` entry would be

`view::widgets_by_field_name` walks `/Fields` as before and then every page's `/Annots` in page
order; a widget no entry reached whose chain names a root no entry reached has that root walked with
no prefix, exactly as an entry of the array is, after the listed ones. A widget whose chain meets no
`/T` stays §12.7.4.2's "simply a Widget annotation". A root already walked whose `/Kids` do not
answer the widget's `/Parent` leaves the widget out, as the `/Fields` walk does. A recovered root
whose qualified name a listed field already has joins that field, which is §12.7.4.2's own rule for
two dictionaries with one name. The `/Parent` walk is bounded by `MAX_FIELD_DEPTH`, since a chain
can be a cycle.

**Why recover rather than read the omission**: the omission is the only one of the three statements
that does not describe a field, and reading it as decisive leaves a page with fields a person can
see and cannot fill. It is the same argument ADR 0736 made for the `/Parent` an entry states, and
the two recoveries rest on the same entry. What the clause still decides: a dictionary on no page
and in no entry stays unread, so a signature field on no page needs `/Fields` (§12.8.1).

**What it changes beyond the page**: a recovered field is a field everywhere the table is read — a
submit-form action exporting every field exports it, a reset of every field resets it, a save
writes its value, a hide action names it, a script's realm holds it.

## 3. The file the class came from, and whose behaviour it was written against

The census's class 5 was `PDFBOX-3094-2.zip-2.pdf`: 273 of its 490 roots omitted, its producer
entry `VF2PDF.dll 2.3.6.0`. Its issue, PDFBOX-3094, says what wrote it: PDFBox 2.0.0's merge of
already-merged forms, reported as "loosing field name values" and "the form is not editable" — so
the last writer's own reading of `/Fields` loses these fields, and the person who reported it
called that a defect. The scripts inside were written by the form's author against Adobe's object
model and name the omitted fields. Neither program is matched; the choice rests on Table 226 and
§12.7.4.2, and the issue is evidence of which reading a person expected.

## 4. The population and the cost

`orphan_field_census` over the script census population (90 260 files on this machine, 7 783 with
an `/AcroForm /Fields` array): **79 documents state a named root `/Fields` omits — 4 482 roots,
5 737 widgets.** Producers include Ghostscript, Quartz, Acrobat Pro DC, Adobe PDF Library, iText,
pdfTeX and Neevia; in 31 of them every root is omitted.

**The launch path walks no other page, and that was measured rather than assumed.** As first
built, every reader of the table walked every page's `/Annots`, and one of them —
`ViewState::apply_resumed`, which a host calls after every command — built the table before asking
whether any run had finished. `ISO_32000-2_sponsored_EC3.pdf` (one `/AcroForm`, no widget, 1 023
pages) then drew page one with 77.4–78.8 MiB allocated against `launch-path.toml`'s 25.5 .. 32.0,
where an export of HEAD drew it with 28.4–30.1 (three children each, `first-page` phase, separate
target directory). So the table now comes in four widths, `view::Omitted`, and each caller asks the
narrowest that answers it:

- **one page's `/Annots`** — `form::fields` (what a host lays controls from and `delegated_widgets`
  takes out of the drawn page), `field_at` under the pointer, and the open sequence and a page
  turn's scripts (`run_open_scripts`, `run_page_scripts`), for the page shown;
- **the calculation order's own entries** — `recalculate_with_runner`, which runs as a runner
  arrives at the open: the root above each `/CO` entry no `/Fields` entry reaches, no page read;
- **`/Fields` alone first** — `displayed_values`, asked on every repaint, which widens to the whole
  document only when a name it is asked is not a listed field, i.e. a recovered field is on screen;
- **every page** — a value typed or committed by name, a reset, an import, a save, a submission, a
  script's late answer: what a person asked of the document.

A realm told of one page's fields at the open is told of each further field the first time a wider
table names it (`Told::names`), so a script never meets a field by one width and misses it by
another twice. Re-measured after the change: EC3 28.2–28.9 MiB, the launch gate 0 outside (EC3
28.453), the Tier 1 column, Tier 0's `script_corpus` and `pdf-model --test corpus` unchanged from
the figures above. Every other reader of the table is an interaction or a save and reads the whole:
measured warm over 192 form documents (the pdf.js test set's and the 89 above), the added cost per
call is a median of 4 µs and a mean of 87 µs, the largest 1.5 ms on `3006044.pdf`, whose 339
recovered roots are the table's own work.
