# 1565 — The archive target and profile are chosen in the window

Slot 2 of batch ninety, 2026-10-15, a host round. ADRs 1966, 1967; no Q file.

**The premise held in part.** `archiving.rs` was the one runner and `profiles::SHIPPED` the one
list, and Shift+A with nothing chosen only said how to restart. **It did not hold for the
password**: no window "already holds" one — the `Secret` moves into `Command::Open` and is dropped
with it, so an encrypted document reached `PasswordRequired` in the converter. **The hypothesis
held**: the card is the restrictions card's shape and the list is read, never copied — each
profile's sentence comes from its own `[profile] sentence` through `Shipped::sentence`.

**Built.** `archiving::pressed` answers `Convert` for a choice the window holds and `Choose`
(`Choosing`: `Target::ALL` × the shipped profiles, keyboard on PDF/A-2b and `refuse-any-loss`) for
none; `quorra` draws `chrome::ArchiveCard` (← → target, ↑ ↓ profile, Enter, Escape, a press
chooses), GTK a drop-down over a `GtkListBox`, Qt a `QComboBox` over a `QListWidget` through the
bridge's `archive_dialogue`; the native two through `NativeWindow::offer_to_archive` and the
provided `archive_chosen`. The choice is kept for the window's life and said. `password::Kept`
keeps a copy per document where a host sends `Open` with a password, forgets it on close or
cancel, and lends it to `Converter::start`/`convert`; the C library's `Session` keeps one too.
The C ABI gained `quorra_archive_profile_sentence` (240 entry points). The §7.6.4 aggregate note
names `Command::Archive` beside `Command::Print` where bit 11 is asked.

**Gates.** rustfmt clean on every touched file. On the batch's final tree: `RUSTFLAGS="-D warnings"
cargo clippy --all-targets` on pdf-transform, viewer-host, viewer-ui, viewer-gtk, viewer-qt,
viewer-ffi: exit 0; `nextest` on the six: 1 064 run, 1 064 passed (the first run, mid-batch, had 2
failures of this round's own — an entry-point count of 239 in a second test, and an assertion on the
word "password" that the report says legitimately — both fixed). `cargo test -p conformance`: 436
passed, 0 failed. `tools/state.sh docs`: exit 0. `tools/state.sh hosts`: Command 37 of 45 reach the
ABI (unchanged), 240 entry points. Drive step 88 (dev binaries, `:165`): 17 works, 0 wrong in the
four windows — chosen on the chooser, kept for a second press, and `drive-password.pdf` opened with
the kept password and reaching `file-structure/no-encryption`. The whole drive of the four windows
(mid-batch binaries): 304 works, 1 wrong — `82-retained-page` in quorra-qt, a photograph timed
after page one's frame, a step that also read wrong in a sibling's drive and touches no archive code.

**Unfinished.** Choosing again in a window that has chosen (no request asked for it); the Qt
list's last sentence sits at the list's lower edge at 560 px. Proposed
habit: a dialogue a drive step answers takes the keyboard on its list, so Down and Enter mean one
thing in all three windows.
