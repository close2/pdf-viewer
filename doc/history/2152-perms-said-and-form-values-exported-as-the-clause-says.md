# Round 2152 — an unbound `/Perms` is said, and form values leave as the clause says

The owner's A953 (closes audit 2131-T01) and audit 2148's §12.7 fix groups.

## What it did

- A953 (ADR 3309): a revision 6 file whose `/Perms` is absent, short or unmarked opens under `/P` as
  written; `pdf_syntax::UnboundPermissions` and `Document::unbound_permissions` say why, and
  `viewer_core::notes::about` puts the sentence in the document report.
- B01, B02 (ADR 3312): `submission::choice_export` and `button_export`, one mapping for HTML Form,
  FDF, XFDF and `xfa_change`; XFA 3.3's `items save="1"` read first.
- B03 (ADR 3310): `write_state` writes the `/AS` `Field::is_on` draws.
- B04 (ADR 3311, model only): `Entered::Selected(widget)`, carried by `viewer-confined`'s protocol.
- B05, B06 (ADR 3311): `set_field` refuses a push-button any value, and typed text that is no label
  into a list box or a combo box with Edit clear.
- B07: `text_shape` reads `FileSelect`. B08: `qualified_name` keeps the PERIOD after an empty `/T`.
- Q974: the B04 audit test and B03's control make the same call and expect different buttons.

## Ledger

No status moves; 7.6.4.3.3, 12.7.4.2, 12.7.5.2.2–12.7.5.2.4, 12.7.5.3, 12.7.5.4 and 12.7.6.2 stay
`implemented` and name the tests that now hold them.

## Gates

**Gates.** Over `-p pdf-syntax -p pdf-model -p viewer-core -p viewer-host -p viewer-confined`: fmt
exit 0; clippy `-D warnings --all-targets` exit 0 (third run; two `too_many_lines` and a
`similar_names` fixed); `pdf-sandbox --bins` exit 0; nextest `--lib --bins --tests` under `xvfb-run
-a`: 4486 run, 4485 passed, 1 failed — the B04 audit test, the manifest's one entry for these crates;
rustdoc exit 0; `fuzz` `cargo check --bins` exit 0. `cargo test -p conformance --no-fail-fast` exit
0, run after this paragraph was written.

## Habit

The manifest the brief called empty held audit 2148's eight entries; read it before planning.
