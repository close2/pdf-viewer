# 695 — The prompt that needed a terminal, and the password one field order kept out of a log

Date: 2026-08-23. ADR [0545](../adr/0545-a-prompt-that-does-not-need-a-terminal.md).

Touched: `crates/viewer-core/src/secret.rs` (new), `lib.rs`, `command.rs`, `viewer.rs`, `open.rs`,
`tests/{headless,selection_census,accessibility_census}.rs`;
`crates/viewer-host/src/password.rs` (new) and `lib.rs`;
`crates/viewer-ui/src/{chrome.rs,software.rs}` and
`src/bin/pdf-viewer{.rs,/app.rs,/dispatch.rs,/window.rs,/overlays.rs,/sidebar.rs,/surface.rs,/composer.rs}`;
`crates/viewer-gtk/src/host.rs`; `crates/viewer-qt/src/{bridge.rs,host.rs}` and `cpp/window.cpp`;
`crates/viewer-ffi/src/{abi.rs,session.rs}`; `crates/viewer-confined/src/protocol.rs`;
`doc/conformance/ledger.toml` (§7.6.4.1), `doc/todo/30-a-native-host.md`,
`doc/state-of-play.md`, `doc/ui-boundary.md`, `doc/environment.md`,
`doc/traps/instruments-and-reports.md` (trap 16), `doc/HANDOVER.md`.

The **fourth** round on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)"*, taking **item 3** of the
ordering ADR 0509 wrote.

## The item

`viewer-ui` answered §7.6.4.1 on `stderr` and `stdin` and called `std::process::exit(1)` where
there was no terminal — the only place in this tree that answered a document on a file descriptor
and the only one that left the process for want of one. So the tier-2 host could not open an
encrypted document from a desktop launcher at all.

**It needed no message**, which is the ninth time since the six-hundred-and-seventh that a feature
landing in every host has needed no channel. What it did need was a type, for a reason that had
nothing to do with the clause — below.

## What the standard decides, and it is three things this round did not expect

**The modal verb is `should`.** *"If this authentication attempt fails, the interactive PDF
processor should prompt for a password."* Both native hosts carried, in quotation marks, *"the
interactive PDF processor shall … prompt the user for a password"* — a sentence ISO 32000-2 does
not contain, with a recommendation upgraded to a requirement and four words added, and
`viewer_host::policy` said the clause "requires" it. `CLAUDE.md` says quotation marks mean
verbatim.

**The quotation gate could not see it**, and that is a limit of the instrument rather than a fault:
it reads rustdoc blockquotes under a citation, and these were `//` line comments. Worth knowing
before trusting a green gate about a quotation.

**NOTE 2 is what makes `exit(1)` a misreading rather than a rough edge.** It describes the
processor that genuinely cannot ask — *"non-interactive PDF readers that do not have a person
running them such as printing off-line or on a server"* — and a window on a screen is not one of
those whatever it was launched from. The obligation is on the *interactive* processor, and this
program was one the whole time; it looked for the person in the wrong place.

The clause states no attempt limit and nothing about what a prompt looks like. Both stay documented
choices and now live in one place.

## What was built

`viewer_host::password` — `Asking`, a closed `Ask`, a closed `Supplied`, one `prompt` format string
and the two sentences. Three hosts held three `PASSWORD_ATTEMPTS`, three counters and three
comparisons; `viewer-ui`'s counted to the same three and then exited. **Neither native host reset
the count on a successful open** — invisible because each opens one document, and wrong since
Annex O's `ef` gives this program a second one.

`viewer_ui::chrome::PasswordCard` — the tier-2 counterpart of `gtk4::PasswordEntry` and a
`QLineEdit` at `QLineEdit::Password`. §12.7.5.3 is the one place the standard says what an
unreadable echo looks like (*"such as asterisks or bullet characters"*), so the card draws the
second of the two examples rather than inventing a third, and it draws them from the **count** of
characters: `Secret::reveal` is called nowhere in the module.

## The password was one field's declaration order away from a launch log

`Command` derives `Debug` and `viewer-gtk` and `viewer-qt` trace a command with
`format!("{command:?}")` cut to 120 characters. `bytes` is declared before `password` and a
`Vec<u8>`'s `Debug` is about five characters a byte — so the truncation cut the line before the
secret. That is an accident. A variant reordered, a truncation widened or a twenty-byte document
would each have undone it in silence.

`viewer_core::Secret` is the type: no `Display`, no `AsRef<str>`, a `Debug` that says how many
characters and not which, a buffer zeroed on drop through `black_box`, and §7.6.4.1's own 127-byte
truncation as the reserved capacity so that a password the standard reads whole never reallocates
into memory nothing can clear. `zeroize` was **not** taken and the ADR says why: a volatile write is
`unsafe`, and buying it from a dependency is principle 3 paid off in another crate's ledger. Five
consumers failed to compile, the fifth use of that mechanism.

**One gate went blind and got a replacement.** `viewer-confined`'s round-trip test compares `Debug`
strings, so a redacting `Secret` hides a transport that corrupted a password into another of the
same length; that field has its own test now.

## The chrome that had a third way not to reach the screen

687 found two. This is the third and it is sharper: `App::present` opened with
`let first = pages.first()?;`, so a window with **no page** drew no frame at all — and a document
that has not authenticated is exactly that window. The card would have been built, held and never
presented, on either surface. `Surface::without_a_page` draws the chrome over
`pdf_render::SURROUND` on both.

**And the new method then got three things wrong that only the screen could say**, each of them a
line the two methods beside it already have:

- it never `adopt`ed the frame the render thread finished, so every tick asked again and reported
  *nothing to show* for ever — **1196 frames in twenty seconds** of a window that should draw once;
- it armed the frame clock unconditionally rather than only where it had asked for a frame, so once
  that was fixed the window **presented at the tick rate for ever**, which is `doc/todo/36`'s fourth
  rule broken by one line. **Eleven frames and one present**, after;
- and an *empty* overlay list still meant "nothing to show", so the prompt stayed on the window
  after the program had said it was done asking. It means "nothing to show" only until this path has
  drawn once — after that it means "take it away" — which keeps the launch path exactly as it was:
  no blank frame in front of page one.

## Two things the toolkits owed, found by pressing the keys

**`GtkWindow::close` fires `close-request` synchronously**, so the flag the Open button set on the
host *after* closing the dialogue was still false when the close handler read it — and every
password supplied through GTK was reported as a decline. The flag lives beside the dialogue now.

**A plain `GtkWindow` binds no key**, and `GtkDialog`, which binds Escape, is deprecated in the
release this crate targets. Without an `EventControllerKey` the only way out of the GTK prompt was
the window manager's close button, and there is none under `Xvfb` — so the other two hosts declined
on Escape and this one did nothing. ADR 0508's rule paying for itself in the other direction: the
check was one key press.

## What was measured

Three release binaries driven under `Xvfb` on `doc/pdf.js/test/pdfs/issue6010_1.pdf` — `/V 2`,
`/R 3`, password `abc`, one of the eight locked corpus documents `pdf-syntax`'s encryption test
records — with plain `xdotool` (XTEST, 683's instrument lesson), rebuilt and installed before every
measurement. Five things, in all three hosts:

- **the prompt reaches the screen**, carrying the same question, the same clause number and the same
  *Attempt 1 of 3* — a `gtk4::PasswordEntry` in a modal `GtkWindow`, a `QLineEdit` in a `QDialog`,
  and the card over the page;
- **a wrong password asks again**, at *Attempt 2 of 3*;
- **the right password opens the document** — `issue6010_1.pdf — page 1 of 1` in both native hosts'
  title bars and the page itself drawn in the tier-2 host, which is the first time this program has
  opened an encrypted document without a terminal anywhere in the loop;
- **Escape declines by name**, saying *the document is encrypted and no password was given*, with
  the process still running;
- **three wrong passwords say *too many password attempts* and stop asking**, with the process still
  running and the prompt taken off the window.

**And the frame count**, which is `doc/todo/36`'s rule and was measured because the new path could
break it: a window holding the card presents **eleven frames, one of them a present**, and then goes
quiet.

**One instrument fact went into `doc/environment.md`**: `xdotool type --delay 80 zzz` puts *two*
characters into a field and `xdotool key --delay 300 z z z` puts three — X folds identical keycodes
that arrive close together. It is the same shape as the wheel-notch note already there, and a
password prompt that shows a bullet per character is the one interface in this tree where it is
visible.

## The gate that failed, and the half-hour it took to prove it was not this round's

`accessibility_census` refuses this tree: *elements placed by their own marks: 93258, and it was
93267*. Nine elements of one document, `issue5481.pdf`, moved from §14.8.3.3's content rectangle to
no place at all; every other count in the census is unchanged, to the element.

**It is not this round's**, and the proof cost a scratch worktree and a lesson. A `git worktree` at
the branch point — `main`'s HEAD, not one line of this round in it — printed **93267 and passed**,
which would have convicted the change. It had no `target-dir` of its own and so built into the main
tree's; pointed at an empty directory instead, **the same worktree at the same commit printed 93258
and failed**, which is exactly what this tree prints. Three runs each way, deterministic both ways.
The ratchet has been broken on `main` since at least the merge of round 686, which is as far as this
went before stopping: a `viewer-core` round owns it, not a UI one.

**The floor was left alone.** A round that lowers a ratchet it did not investigate has removed the
instrument rather than answered it, and `doc/todo/05`'s rule is that a capability count may only
rise.

**What the mechanism is was not established, and trap 16 says so.** `pdf-spec/build.rs` does emit
`cargo:rerun-if-changed` for the Arlington TSVs, `main`'s working tree and submodules are clean, and
no `RUSTFLAGS` differed. What is recorded is the observation and the command behind it.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctests, `check` over the fuzz targets, and `cargo test -p conformance` — the core plus the
conformance gate, which is `doc/todo/02` §2's row for a change to the host crates and the ledger,
plus the two censuses `viewer-core` is under. §5's binaries were rebuilt and installed before any
measurement.

Nothing here can change a pixel a corpus gate rasterises: no `pdf-*` crate, no rasteriser. The one
`pdf-render` name this round touches is `SURROUND`, read and not changed.

## What is left, named rather than left silent

`Event::OpenFailed` and a zero-page document still `exit(1)` in `viewer-ui`. Those are a different
case from this item — a file that cannot be opened at all rather than one waiting to be
authenticated — but they are the same shape of answer, and a round taking `doc/todo/30`'s window
work next should decide whether a window that says why and stays up is the better answer there too.

**Nothing is queued for the owner's measurement loop.** Everything here is a window, a key and a
screenshot, and `Xvfb` answers all three.
