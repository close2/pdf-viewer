# 1976 — A save an XFA form declines is the reader's to allow, and a submit that brings no form data runs `postSubmit`

Session 1976, a single ledger round (K.2), the ledger lane. ADRs 2763 and 2764; no question. **The slice, and why.**
The brief's first two: a save's `cancelAction` and a transmitted submit's `postSubmit` with no answer. Both are what
ADR 2702 left of the pre/post pairs around the reader's two actions (XFA 3.3 pages 388, 389, 400, 402 and 447), and
both cross `viewer-core`, the confined frame, the C ABI and every host, so they are one change to every host.

**The premise held.** `activate_form_before_save` reported the flag and wrote the file; nothing told the core a
transmitted submission ended without form data. Neither was owner-blocked: `CLAUDE.md` settles the shape of a
document's restriction (four levels, asked once, a host-supplied place), and `restriction::Level::verdict` is that place.

**Built.** `Operation::Save` (reader word `save`, wire and C ABI 9, `QUORRA_RESTRICTED_SAVE`) and
`Restriction::SaveCancelled`, handed to the verdict by `Viewer::save` after `/WS` and `preSave`: `off` writes, `warn`
writes and says why, `on` refuses, `ask` holds `Held::Save` for `Command::Answer`. A save not written fires `postSave`
(page 400's cleanup) and not `/DS`. `Command::Unanswered { document }` fires `postSubmit` with no primary event (page
447's step 1, "whether or not the submit succeeded"); confined kind 49, `quorra_unanswered` (366 → 367 entry points,
header, C# declaration, C driver). `viewer_host::submit::Returned::follow_up` replaces `policy_copy` and answers it for
a submission `reply` does not import, through one `form_data` reading; `quorra`, `quorra-confined` and every
`NativeWindow` also dispatch it where they decline or cannot start a submission (page 389's "cancelled or failed").

**Tests.** `script_form.rs`: a `preSave` that sets the flag at each of the four levels, both answers, `postSave` at
every one; `Unanswered` runs `postSubmit`, not at `off`, not for another document. `viewer-host submit.rs`: seven
endings, the follow-up agreeing with `reply`, an import's ending in nothing. **Plants**, each applied and reversed:
the reason dropped, the cleanup dropped, the handler emptied, an unsent submission given no follow-up — each fails
its test; reversed, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` of `pdf-model`, `pdf-transform`, `pdf-vfs`,
`viewer-confined`, `viewer-core`, `viewer-ffi`, `viewer-host`, `viewer-ui`, `viewer-gtk`, `viewer-qt`: exit 0. nextest
of the ten (after `cargo build -p pdf-sandbox --bins`): 4767 of 4768, 53 skipped; the one, the C driver's window test,
timed out waiting on the build directory my own `script_corpus` walk held, and passed alone, 8 of 8. `cargo test -p
conformance` after `git add -N`: exit 0. `state.sh --round 1976 docs`: exit 0. Tier 2 under `bounded.sh
--lock`: `pdf-model script_corpus` 0 (67 445 PDFs walked, safedocs' 65 968 among them; tika-issue-tracker and
openpreserve absent), the `viewer-core` censuses 0 and 0, `viewer-confined awkward_classes` 0, `pdf-transform gate`
101 on its floor, 20.6 pages/s against 40, as round 1955 read it here (20.2): no difference; `viewer-ui
launch_path` 101, rustc's SIGSEGV compiling the test before it ran, owed to the clocks round. The WinUI host is not built here (no .NET SDK);
`header_and_interop_agree` passes.

**Left of the row**: the other events; the `xdp` and `xml` packagings; an `execute`'s WSDL call; XML signing and
encryption; a submit at `off`; `commitKey` on a choice list's item click, a script's `setFocus` and a page turn; the
merge's format tests at `off`; RFC 0011 step 5's events.
