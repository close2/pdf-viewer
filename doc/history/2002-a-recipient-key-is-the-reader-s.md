# 2002 — A recipient key is the reader's

Session 2002, a single ledger round (§7.6.5.1), the second ledger lane. ADR 2822.

**The slice, and why.** ADR 2801's item 1, the last of its order: the host input of a
`RecipientKey`, without which §7.6.5.1's one `shall` ran for no reader.

**The premise.** Held: unbuilt, effort only, no owner word needed (A800 item 1). One part did not
hold as briefed: "on the open command" is 261 constructors, so the key is a policy command held
for the window (ADR 2822 decision 1), and it is not `Secret` but `RecipientSecret` (decision 2).

**Built.** `Command::Recipient(Option<RecipientSecret>)`, held by `Viewer`, read by `Open::read`
only after `SyntaxError::RecipientRequired`; `Event::RecipientRequired` is §7.6.5.1's prompt, a
key that does not open an `OpenFailed` in `RecipientKey`'s words; confined frame 53, event 30.
`viewer_host::policy::recipient_key` reads `--recipient-certificate` and `--recipient-key` (DER or
PEM; an encrypted key refused by name), sent as a fifth reader command, which every window reaches
with no edit; each says the prompt with the two words. C ABI: `quorra_recipient_key`, event 31, reader words 6 and 7, counts
372/353/32, the C# layer. `fuzz_seeds` counts only hexadecimal constants as certificates now, and
`awkward_classes` counts the prompt as a refusal.

**Tests.** `viewer-core/tests/recipient_key.rs` (six: no key asks; the named certificate and its
key open s4; another certificate asks; the right certificate with another key is `RecipientKey`;
`None` withdraws; an EC key opens `key-agreement-s5.pdf`), the protocol round trip, four host
tests. **Plant**: the recipient arm disabled and the frame's halves swapped fail four; reversed,
they pass. **Drive**: `quorra` and `quorra-gtk` under Xvfb (lavapipe), `PDF20_AN001-BPC.pdf` then
`adbe-pkcs7-s4.pdf`: without the words "could not be opened ... §7.6.5.1", with them "opened beside".

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (the six viewer crates and
`conformance`, `CARGO_INCREMENTAL=0` after a rustc ICE): exit 0; nextest of the six under
`xvfb-run`: 1162 of 1163, the C program's event count (fixed; its two tests and the recipient
tests 21 of 21 on re-run); `cargo check -p viewer-qt --all-targets` and `cd fuzz && cargo check
--bins`: exit 0; conformance after `git add -N`: exit 101 on `batch` (a detach timing at load 350)
and `records` (this record unfinished), both exit 0 re-run; `state.sh docs` exit 0 (run at the
merge), and rustdoc `-D warnings --document-private-items` on the six: exit 0.
Tier 2 by the map: `selection_census`, `accessibility_census`, `awkward_classes` exit 0 each (974,
1020 documents; openpreserve and tika absent). `corpus`, `raster_golden` not run: no file of
`pdf-syntax` or `pdf-model` changed. `launch_path` not run (load above 270).

**Left of the rows.** §7.6.5.1 is `implemented`; §7.6.5 and §7.6 stay `partial` on §7.6.5.3:
RSAES-OAEP, ECMQV, the Triple DES key wrap and brainpoolP512r1, effort only.
