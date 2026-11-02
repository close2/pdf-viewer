# 1906 — A script reads the event it runs under, and the pointer crossing a field runs its `mouseEnter` and `mouseExit`

Slot 6 of batch 131, 2026-11-02, Opus 5.5, two sessions (the first handed over at its context bound). ADRs 2646
and 2647; Q782 unused. Ledger: §K.2 stays `partial`; its note lists `$event`, `mouseEnter` and `mouseExit`.

**Built.** XFA 3.3 pages 398–404's `$event`: `pdf-model`'s `xfa/calculate/event.rs` holds the primary event
and `Form::resolve` answers `$event` and `xfa.event`, `target` the container and every other property a node
past the template's containers, so the wire to the script worker is unchanged. `Session::fire` sets the event
before each object's own events of the eight activities a person causes, and the listeners above it and the
calculations it wakes keep it; a merge, `initialize`, `ready` and `docReady` run under none. Page 398's table
decides what is set (nothing for an exclusion group), an unset or unsupplied property is refused by name, a
write other than to `change`, `selStart` or `selEnd` is refused as read-only (page 399), and a `newText` the
`change` scripts moved is placed into the field before the calculations (page 400). `Stimulus::Entry` carries
what each field showed, recorded at `begin_typing`, as `prevText`. The JavaScript realm registers `$event`, its
`get` and `set` questions to the host, and `xfa.event` returns it. `Stimulus::MouseEnter` and `MouseExit`
(page 391) are the field's own activities; `viewer-core`'s `Open::hovered` follows the pointer whatever the
button (unlike `inside`, held by §12.6.3's first constraint) and raises the exit before the entry, before the
`/X` and `/E`; a form that states neither event runs no activation at a crossing (`Resource::has_activity`).

**Premise.** Held: one object per dispatch reached through the paths both realms already ask. Not held as
written: an entry carried no previous text, and the crossing events ignore the button, so they needed a
pointer record of their own.

**Plant.** A `mouseEnter` running `for (;;) {}` and `while (true) {}` in the confined worker is stopped inside
five seconds, what it set dropped, the report naming the event (`a_mouse_enter_event_that_never_ends_is_stopped`).

**Gates.** `cargo test -p pdf-script-worker --features engine` 0 (55 passed); `-p viewer-core` 0 (436, 2
ignored); `-p pdf-model --test xfa_datasets` 0 (18); `-p pdf-formcalc` 0 (75); `-p pdf-script` 0 (23); `cargo
clippy -p pdf-model -p pdf-script -p viewer-core -p pdf-formcalc --tests --no-deps -- -D warnings` 0 and `-p
pdf-script-worker --features engine` the same 0; `--bin cited` 0; `--bin quotations` 0; `cargo test -p
conformance --no-fail-fast` 0 (470 passed, no red); `rustfmt --edition 2024 --check` on my files 0.
