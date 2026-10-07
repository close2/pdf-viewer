# 1414 — One widget of a field is a `Field` of its own, and a style redraws the check box glyph

Slot 1 of batch sixty-five, 2026-10-07, a script round. ADRs 1664 and 1665; ADR 1652 marked
superseded in part. No ledger row moved (§12.5.6.19, §12.7.4.2 and §12.7.5.2.3 are `implemented`,
§12.6.4.17 stays `out-of-scope` on Q286); no question written.

**Premise.** Half held. `doc/todo/56` class 7 and `bridge.rs`'s `app.fs` comment were as the brief
said. But a field's state does not cross in `pdf_model::view::Request` (§12.6.4's action requests,
78 `Request::` sites outside `pdf-sandbox`) nor in `viewer-confined`'s protocol, which carries no
script type. It crosses in `pdf_script::Request` and `pdf_script::wire`, so those were the sites.

**Widget addressing (ADR 1664).** `FieldState` now holds the field's members and a `WidgetState`
per widget, in the field table's `/Kids` order. `getField("x.N")` answers a `Field` of that widget:
its widget members read and write that widget, its field members and its value the field. Past the
last widget it answers `null`. `setFocus` through a widget after the first is refused by name.
`ScriptEdit::Property` gains `widget`, and the wire is version 6. On a save, a one-widget `textColor`
or `alignment` writes no `/DA` or `/Q` on the field, where its siblings would inherit it. With that
filter planted off, `a_property_set_on_one_widget_reaches_that_widget_alone` fails.

**Style and `app.fs` (ADR 1665).** `Field.style` is `/CA` holding Table D.6's code for the solid
glyph. A toggling button whose script set a style is constructed anew, and its saved states are
owed (`unconstructed`). Rendered at 6 px/unit and looked at: ✘ ★ ● inside the border. Where the
`/DA` font is not `ZapfDingbats` the style is reported and not drawn. In `6942042.pdf`, the census's
one use, all four boxes inherit the form's `/Helv`, so the report fires there.
`app.fs` is refused by name, with its own `EXCLUDED` row.

**Found, not changed.** `/NeedAppearances` is written only for an indirect `/AcroForm`.
**From slot 6.** The three column headers now spell `tools/bounded.sh --lock`; `bounded.rs`'s list
keeps RFC 0008 alone.

**Gates.** `rustfmt --check` on the 21 files: exit 0. `-D warnings` clippy on `pdf-model`,
`pdf-script` with and without `engine`, and `pdf-script-worker` with `engine`: exit 0 each. nextest:
`pdf-model` 1 973 passed outside the two walks (38 decoder tests had failed before the sandbox
worker was built, trap 10), `pdf-script` with `engine` 100, without 22, worker 25. `cargo test -p
conformance`: exit 0, `--test bounded` 5 passed. Behind the lock, one hold of 383 s, peak 1.66 GiB:
the Tier 1 column ran 21 101 runs (11 776 finished, 9 317 threw, 8 unparsed, 0 over a budget), against
21 100, 11 773 and 9 319 before. `HELD_THREW` 9 319 → 9 317 and `HELD_RUNS` 21 100 → 21 101. The worker
column gave the same five figures with 0 lost. Tier 0's `script_corpus`: 356 held, 0 moved. The
`pdf-model` corpus: exit 0, every ratchet at slack 0. `launch_path` not counted, because
`field_state` runs only with a runner and the open path without one is unchanged. Duration 3 571 s.
