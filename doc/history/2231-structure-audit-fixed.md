# Round 2231 — audit 2229's fixes: an XObject an /OBJR names, /Alt once, a Span's language, /TU, an outline row's language

A fix round in the `audit` worktree for review 2229 (§14.6 to §14.9): five B findings with six
failing tests and one D finding, in the four groups the review suggested.

## What changed

- B01, D01 (ADR 3625): a `Do` of a form or image `XObject` whose `/StructParent` the tree claims
  records an `ObjectSpan` (`ObjectItem::XObject`, with its extent and refusal); `by_absence`
  subtracts it, `Tree::logical_text` places it where its `OBJR` stands, and viewer-core's walk
  attributes it as a `ContentItem::Object`. Divided pieces carry the spans like `marked`.
- B02, B05 (ADR 3626): `structure::enclosing` reads `/Alt` and `/E` outermost-first beside
  `/ActualText`; `described_elements` speaks an element's phrase once. A structured sequence with no
  language in its chain takes the catalog's (or the unknown language), never a `Span`'s.
- B03 (ADR 3627): `submission::compose`'s sentences, `field_name_of` and the script question name a
  field by `FieldName::shown`; `view::field_name` is the new public lookup.
- B04, outline half (ADR 3627): `Outline::language`, across the confined wire; `PanelRow::language`;
  GTK's Pango language attribute; `quorra_outline_language` in the C ABI and the C# interop.

All six audit tests pass; the manifest lists none; 2229's B04 stays open for its layer half.
Review 2197's `outline_item_style_spec_audit.rs` gained `language: None` in an `Outline` literal.
Ledger: 14.7.5.3, 14.7.5.4, 14.8.2.2.2, 14.8.2.5.1, 14.9.2.3, 14.9.3 re-read, `implemented`;
14.9.2.1 `partial` (optional content group names), §14.9 and §14.9.2 derived `partial`.

**Gates.** fmt exit 0; clippy `-D warnings --all-targets -p pdf-model -p viewer-core -p
viewer-host -p viewer-qt -p viewer-gtk -p viewer-ui -p viewer-ffi -p viewer-confined` exit 0.
`pdf-sandbox` 0. nextest `--lib --bins --tests` of the eight (`xvfb-run -a`): 4620 run, 4617
passed, 3 failed — viewer-ffi's entry-point counts and the C# interop, for `quorra_outline_language`,
then viewer-ffi's 97 of 97 passed; `audit-diff.py` against the emptied manifest: nothing else.
rustdoc `-D warnings --document-private-items` of seven, private target, exit 0. `fuzz` `cargo
check --bins` exit 0. `cargo test -p conformance --no-fail-fast` exit 0 after the frontier map,
the derived heads, the record's gates and one `// not a repair:` marker.

## Left

1. `layer_rows`' language: `Answer::Layers` carries no document language (ADR 3627, "Left").
2. An outline title's own §7.9.2.2.2 escape language in its row.
3. Qt and quorra present no per-row language (no toolkit hook; quorra's tree holds no panel rows).
