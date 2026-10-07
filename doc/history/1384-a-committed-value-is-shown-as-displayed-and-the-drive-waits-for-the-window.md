# 1384 — A committed value is shown as the field displays it, and the drive waits for the window

UI slot of batch sixty. ADRs 1604, 1605; no row, no question.

**Displayed value.** `FormField::displayed` is `ViewState::displayed_value`'s answer beside the
characters, on the confined wire and as `quorra_field_displayed`. GTK and Qt text controls show it
without the keyboard (on open, after a commit) and the characters with it, because `/K` would
refuse typing that began from `$12.50`; Enter commits and hands the keyboard to the page. Asked one
field at a time it cost `prefilled_f1040.pdf`'s 116 fields 0.6–0.9 → 24.5 ms per `Query::Fields`
(every toolkit repaint) and `160F-2019.pdf` 0.54 → 5.2 ms; a `/F` prefilter and one batched
`ViewState::displayed_values` (additive, in 1383's `view/scripts.rs`) bring them to 0.80 and
0.41–0.85 ms (`examples/fields_cost.rs`). Drive step `39-field-shown` reads `$12.50 $7.00 $19.50`
off each toolkit's controls, then `12.5` once Price1 holds the keyboard (ADR 1604).

**First present.** 1383's ADR 1602 names `run_open_scripts`: it is `Command::Presented`, sent by
all four windows beside `Command::Report` under `viewer_host::report::Due`, once a document, on the
wire and as `quorra_presented`; `tests/open_sequence.rs` holds it (ADR 1604 section 5).

**Drive.** `launch` waits for each window's first-frame line under `--trace=launch` (ceiling 30 s)
instead of `sleep 5`; inputs the core traces wait for their line (Tab, `SetField`, save, search's
end, the refusals step 37 judges), titles and AT-SPI nodes are polled, photographs retaken until the
colour lands. Untraced inputs keep their settle, each reason beside it (ADR 1605). Behind the lock,
before (batch-opening binaries and script): 141 works / 0 wrong / 3 not offered, 1129 s over the
time column, 1135 s wall. After: 143 / 0 / 3, 440 s, 442 s, no ceiling reached. Groups: `23`
82.5 → 29.5 s, `33` 74.9 → 18.0 s, `27` 13.4 → 5.2 s, `25` 148.1 → 72.8 s, `37` 90.5 → 29.8 s.
Threads under the agent's user at the first heavy run: 140.

**Left.** An editable combo box's text keeps the value (no commit wiring of its own);
`AccessibilityNode::value` still carries the characters, so `quorra`'s own node of an unfocused
field reads `12.5` — `viewer-core/src/accessibility.rs`, not this round's.

**Gates.** `rustfmt --check --edition 2024`, each touched `.rs` file by name: 0. `RUSTFLAGS=-D
warnings cargo clippy --all-targets` on viewer-core, viewer-confined, viewer-ffi, viewer-gtk,
viewer-qt, viewer-host: 0; viewer-ui `--lib --bins`: 0 (its `launch_path` test, a sibling's, is
over clippy's line limit). `cargo nextest run` on those six: 0, 703 passed; viewer-ui: 0, 148;
pdf-model `--lib --test save_round_trip --test aform`: 0, 576. `cargo test -p conformance
--no-fail-fast`: 392 passed, its one failure this record's own draft, then 0. The drive behind
the lock: 0, 143 works.
