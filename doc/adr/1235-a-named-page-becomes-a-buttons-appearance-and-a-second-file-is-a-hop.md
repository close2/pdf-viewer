# 1235 — A named page becomes a button's appearance, and a second file is a hop this crate cannot take

Status: **accepted**.
Context: `crates/pdf-model/src/named_page.rs` (`page_as_form`, `rotation`, `Reference`),
`crates/pdf-model/src/forms_data.rs` (`FdfField::appearance_reference`, `read_field`,
`APPEARANCE_STATES`, `Import::appearance_reference`, `match_fields`),
`crates/pdf-model/src/view.rs` (`NamedPageAppearances`, `annotation_count`, `ViewState::import`,
`ViewState::append_templates`, `Imported::refused`),
`crates/pdf-model/tests/forms_data.rs`.
Builds: ADR 1186 (what crosses from another file, and what only names one), ADR 1223 (a value that
crosses is copied), ADR 1155 (a file a document named comes from a directory a person supplied),
ADR 0090, ADR 1070.
Amends: nothing.
Clauses: ISO 32000-2 §12.7.8.3.2 (Table 249's `/APRef`), §12.7.8.3.3 (Tables 252 and 253),
§12.7.7 (named pages), §12.5.5 (Table 93, the appearance algorithm), §14.11.2.1 (the crop box),
§7.7.3.3 (Table 31), §7.7.3.4 (inheritance), §7.8.2, §11.4.7, §12.7.6.4.

## The entry has two branches and the ledger had been treating them as one

Table 249 calls `/APRef` "[a] dictionary holding references to external PDF files containing the
pages to use for the appearances of a push-button field", and the word *external* is what kept it
unapplied for as long as it was unapplied. But the entry's values are Table 253 named page
references, and that table makes the file **optional** with a sentence about its absence:

> If this entry is absent, it shall be assumed that the page resides in the associated PDF file.

So one branch names a page in a second file — §12.7.6.4's hazard, whose bytes may come only from a
directory a person supplied — and the other names a page in the document being drawn. Only the
first is a host question. The second needs nothing this crate has not got: §12.7.7's two name
trees were already read for §12.7.8.3.3's templates.

## The no-`/F` branch: a page becomes a form

§12.7.7 states the purpose itself — "[a]n import-data action can add the named page to the
document into which FDF is being imported, either as a page or as a button appearance" — and the
second half is this. A widget's appearance is a form XObject, because §12.5.5 maps "from the
coordinate system of the appearance XObject (as defined by its Matrix entry …) to the annotation's
rectangle in default user space". So the operation is page → Table 93 form, and every entry of it
follows a clause rather than a choice:

- The **stream** is §7.8.2's concatenation of the page's `/Contents`, which is what the interpreter
  already reads a page's marks from.
- The **`/Resources`** are the ones §7.7.3.4's inheritance puts in effect. A name in the `/Pages`
  tree is a page of the page tree and inherits up its `/Parent`; a `/Templates` name "shall have no
  Parent" and inherits nothing. `Pages::get` is the first reading, `Pages::detached` the second,
  and taking the second for both would silently empty a visible page's resources.
- The **`/BBox`** is the crop box, which §14.11.2.1 makes "the region to which the contents of the
  page shall be clipped (cropped) when displayed or printed" — the same clipping Table 93 gives a
  form's bounding box, already defaulted to the media box and intersected with it.
- The **`/Matrix`** carries Table 31's `/Rotate`. The entry is a multiple of 90, so the rotation's
  terms are 0, 1 and −1 exactly and no trigonometry is evaluated. No translation is owed with it:
  §12.5.5's step 1 takes the bounding box *through* the matrix and bounds it upright, and its steps
  2 and 3 map that onto `/Rect`.
- The **`/Group`** is copied where the page states one, because §11.4.7's page group and Table 93's
  group attributes dictionary are the same dictionary (Table 31 points at both clauses), and a page
  whose marks the producer composited inside a group composites differently without it.

**Nothing crosses a file boundary here, so ADR 1223's copy is not what this needs.** The page is
the target document's own; its resources name the target document's own objects and stay
references. What that rule decides instead is the *other* branch, below.

**The result goes into `Import::appearance`, the entry a stated `/AP` fills.** Table 249 ranks the
two itself — "[t]his entry shall be ignored if an AP entry is present" — so they are one value with
two spellings, and a second route to the widget could only disagree with the first. Reusing it
means the resolved `/APRef` is drawn, saved into §7.5.6's update and undone by the code that
already does those three, with no new path anywhere.

**A page's `/Annots` are not drawn into it, and the import says so.** Table 31 keeps them beside
`/Contents` rather than in it, and §12.5.5 draws an annotation against the page it is on. Composing
them into a form would be this program deciding a composition the producer did not write, which is
the far side of `CLAUDE.md`'s provenance line; a named page carrying them gets a sentence on
`Imported::refused` instead.

## The `/F` branch: what is missing is the hop, and it is named

This is not an impossibility and the ledger's rows no longer call it one. Both halves the branch
needs exist: `forms_data::carry` turns a value that crosses a file boundary into a tree of direct
objects (ADR 1223), and `page_as_form` turns a page into a form. What is missing is the **hop**.

An import already suspends on one host question: `viewer_core` raises `Purpose::ImportData`, the
host answers it from `viewer_host::policy::read_import`'s person-supplied directory, and
`interact::import` opens the bytes and applies them. `/APRef`'s `/F` — and Table 253's `/F` one
subclause over, which is §12.7.8.3.3's identical entry — is a **second** question raised *while the
first is being applied*. That is §12.6.4.4's suspended-walk shape, which `Purpose::TargetRoot`
already has in that crate.

It is not built here because `pdf-model` has no filesystem and must not acquire one: that is the
boundary ADR 1155 fixed and the reason a document-named file cannot be opened from inside the model
at all. Naming the shape precisely is what this round owed, so that the round that builds it in
`viewer-core` is not re-deriving the design. Until then the unresolved reference stays on
`FdfField::appearance_reference` and its reason on `Imported::refused`, so a person importing such a
file is told which button kept its own artwork and why (trap 5).

## Cost

One conversion this program performs that no clause spells out operator by operator — but every
entry of it is a clause's, and the marks are the target document's producer's, carried whole. The
choice that is a choice, and is recorded as one: a named page's annotations are not composed in.
