# 1961 — An XFA event's submit is a submit-form action, and its execute is named

Session 1961, a single ledger round (K.2), the ledger lane. ADR 2738; no question.

**The slice, and why.** An event's properties other than `script`, which the brief put first: page 703's
`submit` and `execute` (XFA 3.3 pages 445–453, 722–723, 858–861). In a conforming hybrid the widget carries no
action where the XFA resource gives the field actions (Annex K), so a template's `submit` is the only statement
of a submit button, and every one was reported "not performed". `commitKey` 2 and 0 touch every host and the
confined protocol and are a slice of their own.

**The premise held.** `Performs::Unperformed` named all four properties; nothing composed or sent them. Effort was
the only obstacle: page 859's `pdf`, `xfd` and `urlencoded` are Table 240's bits 9, 6 and 3, which
`submission::compose` already writes, and A98's submit level already governs the send. No Q804.

**Built.** `xfa::submit` reads a `submit` (format, `xdp` by default per page 858's underlining and page 597; target;
textEncoding; `signData`/`encrypt`/`encryptData` children) and an `execute` (connection, runAt). `Submit::action`
states the `SubmitForm`: `/F` the target, bit 9, 6 or 3, bit 2 (page 445's variable data), POST; another
textEncoding is owed. The activation holds it as `xfa::FormSubmit` (at most 4) with the clicked field; the view
state holds it; `Viewer::handle` takes it (`send_form_submits`) and runs `preSubmit`, `compose`, `Event::Submit`,
and `postSubmit` where nothing was sent, the submit-form action's path (`interact::carry_out`). Said, not sent:
`xdp`, `xml`, `formdata` (page 859's unnamed legacy software), an unlisted format, no target (page 860), signing
or encryption asked (page 453 steps 3–4), a submit under `preSubmit`/`postSubmit` (a choice: page 446 is silent).
`execute` with `runAt="server"` does nothing (page 722); any other is named, no connection as page 722's error.

**Tests.** `cascade.rs`: seven (three formats and their flags, four packagings refused, signing/encryption/no
target, the encoding owed, `execute` three ways, the pre/post-submit guard, the bound) and the hundred-thousand
test re-pointed. `script_form.rs`: a click hands the host a PDF POST after `preSubmit` set a field, and nothing at
`off`. **Plants**: `pdf` mapped to bit 6 fails the format test and the window test; the guard disabled fails its
test; restored, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-model`, `pdf-formcalc`, `viewer-core`): exit 0.
nextest of the three: 2940 of 2940. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh --round 1961
docs`: exit 0. Tier 2: `-p pdf-model --test script_corpus` under `bounded.sh --lock`, exit 0 (25 held documents on
disk, 0 moved; 331 not on this machine); `viewer-core`'s `selection_census` and `accessibility_census`, exit 0 each;
the engine column not run, failing at HEAD. `cargo build` of `viewer-ui`, `viewer-gtk`, `viewer-qt`, `viewer-ffi`,
`viewer-confined`, `viewer-host`: exit 0; the WinUI host is not built here, and no host's source changed.

**Left of the row**: the other events; the `xdp` and `xml` packagings; an `execute`'s WSDL call; XML signing and
encryption; a submit at `off`, where no runner activates the form; `commitKey` 2 and 0; the merge's format tests
at `off`; a save's `cancelAction`; a transmitted submit's `postSubmit` with no answer; RFC 0011 step 5's events.
