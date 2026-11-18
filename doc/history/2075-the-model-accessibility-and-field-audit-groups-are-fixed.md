# 2075 — The model's accessibility, field and page-boundary audit groups are fixed

Session 2075, an audit-fix round in `pdf-viewer-gwg`. Groups G017 to G020 of
`doc/reviews/2038-specification-audit/fixes.md`: 35 findings (18 B, 14 T, 3 D).

- **Fixed 35, argued 0, owner 0, open 0.** Each line of `fixes.md` names its ADR and tests; the 31
  manifest failures of the four groups pass unchanged and leave `expected-failures-current.json`.
- ADR 2976: an empty `/Lang` is a stated unknown that shadows enclosing languages; an empty
  `/ActualText` replaces with nothing; a structure element's `/ActualText` covers everything it
  encloses, once (`structure::enclosing`, `ParentTree::element_id`); `/Alt` and `/E` escapes are
  spoken in their language (`Described::phrase_languages`), superseding ADR 2812 §2's last sentence.
- ADR 2977: Errata Collection 3's Issue #200 in the header search.
- ADR 2978: `AccessibilityNode::print_field` carries Table 383 to a host, across the confined protocol.
- ADR 2979: `/TI`, `/I`, `/RV`, `/DS` and a choice field's `/Opt` stop at the terminal field; a `/V`
  naming one label twice keeps both `/I` items.
- ADR 2980: a script's `readonly`, `required` and `charLimit` stand over the file's for the form
  description, the submission's bit-2 check and a later value.
- ADR 2981: the page tree's depth bound rises to 1024 with a set-backed cycle test; the structure walk
  reports both of its cuts; a boundary wholly outside the medium clips everything; the name-tree bound's
  comment says what it is.
- Owner audit files edited: `named_property_actualtext_spec_audit.rs`, rustfmt only (A868, A875).
- Ledger: §14.9.2.2's test names follow the audit's (main's last failing conformance check); notes
  of §7.7.3.1, §14.8.4.8.3, §14.8.5.6, §14.9.2.2, §14.9.4 and §14.11.2.1 restated. No status moved.
- Trap: a `#[expect(too_many_lines)]` precedent covers a field-per-line constructor, and adding a
  field to `Interpreter` pushes `for_page` over the line count; read `checkpoint`'s reason first.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 over pdf-syntax, pdf-model, viewer-core,
viewer-confined and viewer-accessibility; their nextest 31 failed, exactly the manifest's entries less
the 31 this round fixed; rustdoc `--document-private-items` 0; fuzz `cargo check --bins` 0;
conformance 0 (main's five failing checks are gone, §14.9.2.2's row being this round's).
