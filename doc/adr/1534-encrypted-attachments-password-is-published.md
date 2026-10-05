# 1534 — `encrypted-attachment.pdf`'s password is published, so the corpus gates answer it

Session 1349. Status: accepted. Amends ADR 1377 (its table, nine rows to ten). Leaves ADR 1040
standing. Code: `crates/pdf-model/tests/support/corpus_passwords.rs`; `LOCKED` in
`crates/pdf-model/tests/corpus.rs`; `NO_RENDER_NEEDS_A_PASSWORD` in `tests/oracle.rs`;
`REFUSED_OPEN` in `tests/save_round_trip.rs` and in `viewer-core`'s `tests/accessibility_census.rs`;
`NOT_COMPARABLE` in `crates/render-raster/tests/corpus.rs`.

## 1. The premise that did not hold

Every gate called this document's password "published nowhere". It is published in pdf.js's own
unit tests: `test/unit/api_spec.js`, "gets encrypted attachments in password-protected documents",
opens the file with `000000`. The test before it hands `auth-event-ef-open.pdf`'s attachment the
same password. `examples/encryption_census --password encrypted-attachment.pdf=000000` opens it
(`/V 5 /R 6`) and matches the password as the **owner** password, which §7.6.4.1 accepts. ADR 1377's
rule covers a password typed into pdf.js's own tests (`print_protection.pdf` is the precedent), so
the row is added under that rule.

## 2. What does not change

ADR 1040's reading stands. The file states no `/AuthEvent`, so §7.6.6 Table 25's default `DocOpen`
wants the key at the open, and `Document::open` without a password still refuses it with
`PasswordRequired`. `pdf-syntax`'s `the_same_document_without_it_asks_for_the_password_doc_open_requires`
pins that. The gates now answer the prompt the way a person would.

## 3. What the references do

Asked with the password (`pdftoppm -upw` and `-opw`, `mutool draw -p`, `gs -sPDFPassword=`), all
three draw 612x792 with the one word *Example* (mutool's raster looked at). Each list that held the
page as unopenable loses it and states why; each walk that now opens it either judges it like any
other page or moves its count with this ADR named beside the bound.
