# 1990 — A drop-down list's `preOpen` and `postOpen` run

Session 1990, a single ledger round (K.2), the ledger lane. ADRs 2791 and 2792; no question.

**The slice, and why.** All round 1986 left, page 392's `preOpen` and `postOpen`, core and windows
together: the core alone would be a stimulus nothing raises.

**The premise.** Unbuilt, effort only. ADR 2784 corrected twice: the confined window draws no
drop-down list, and the ABI has two surfaces (four entries). A `preOpen`'s `cancelAction` takes
ADRs 2763/2777's path: the reader's level, here for `fill`.

**Built.** `Stimulus::PreOpen`/`PostOpen` on a `choiceList` whose `open` is absent, `userControl`
or `onEntry` (pages 392, 653), each under an action event clearing `cancelAction` (page 402) and
setting no primary property (page 398). `Command::OpeningList` runs `preOpen` and grants with
`Event::ListOpens`; `Command::OpenedList` runs `postOpen`. `Restriction::OpeningCancelled` against
`FillInForm`, `Held::OpeningCancelled`, page 400's cleanup where the list is withheld. Hosts:
`quorra` asks at the press on a combo box; `viewer-qt`'s `DropDown` at `showPopup`; `viewer-gtk`
(popover `visible`) and WinUI (`DropDownOpened`) take the list down, ask, and show it at the grant;
the confined window says a grant it never asked for. Wire kinds 50, 51 and event 29; C ABI
`QUORRA_EVENT_LIST_OPENS`, `QUORRA_OWED_LIST_OPENS`, four entry points (367 -> 371), C# interop.

**Tests.** `cascade.rs`: the five `open` values (three fire, two say why), `cancelAction` set and
clear, `$event.name` refused under `preOpen`. `script_form.rs`: `preOpen` before the grant and
`postOpen` after the display, at `off` neither; the four levels for `fill` with the cleanup, and a
non-cancelling `preOpen` restricting nothing at `on`. Wire round trips; C driver counts.
**Plants**, together and reversed: `always` taken as a drop-down, the cancel not kept, `name` set
under `preOpen`, the refusal's cleanup dropped, the grant withheld at `off` — all five tests fail;
reversed, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` of `pdf-model`, `pdf-formcalc`,
`viewer-core`, `viewer-confined`, `viewer-ffi`, `viewer-host`, `viewer-gtk`, `viewer-qt`,
`viewer-ui`, `pdf-transform`, `pdf-script-worker`: exit 0; nextest of the eleven: 4847 of 4847, 51 skipped;
conformance after `git add -N`: exit 0; `fuzz cargo check --bins`: exit 0; tier 2 (the
`viewer-core` censuses, `viewer-confined awkward_classes`, `pdf-model script_corpus`): exit 0 each (script_corpus 67 445 walked, safedocs' among them, tika and openpreserve absent, 4221 s);
`state.sh --round 1990 docs`: exit 0. `launch_path` not run (load 120–450); WinUI not built (no
.NET SDK), `header_and_interop_agree` passes; no host driven under Xvfb.

**Left of the row**: nothing owed. K.2 and K move to `implemented`; what remains is refused by name
or not owed (`indexChange`, the WSDL and signing events, an `execute`'s call, XML signing and
encryption), and `$host.openList()` is described outside XFA 3.3, as `addItem()` is.
