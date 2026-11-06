# 1959 — A field's events read the keys held, and its change the whole text typed

Session 1959, a single ledger round (K.2), the ledger lane. ADR 2732; no question.

**The slice, and why.** The key state, as the brief asked: `$event.modifier` and `shift` on a field's eight
events, `keyDown` and `fullText` on its `change` (XFA 3.3 pages 398, 402–403), the four round 1956 left. Enter and
Escape for `commitKey` stay out: each needs a host gesture that does not exist.

**The premise did not hold.** The brief priced 82 `Command::Pointer` constructions and the confined protocol, but
`Command::Keys` already carries `view::Keys` from `viewer-ui`, `viewer-gtk`, `viewer-qt` and `viewer-ffi`'s
session ABI before the event, and across the confined protocol (ADRs 1762, 1771, 1786): the one place exists. The
Windows host's window ABI had no keys entry. Effort was the only obstacle; no Q801.

**Built.** `Resource::activate_held` takes `Option<Keys>`, and the view state passes its own. A field's `click`,
`enter`, `exit`, `mouseDown`, `mouseUp`, `mouseEnter`, `mouseExit` and `change` set `shift` and `modifier` (page
403). A subform's events and a field's `validationState` do not. A window that did not say is refused by name, and
page 399 refuses a write. `fullText` (page 402) is the whole text the window handed the field, held by `set_field`
before `accepted` cuts it to the field's room. `keyDown` is `Keys::arrows` at that edit. Both are `xfa::Typing`,
handed to the commit's `change`, since the arrow and the paste belong to the edit, not the commit. `viewer-ffi`:
`Window::keys`, `quorra_window_keys` and the header line. `Window::press` tells the press's own modifiers. The
WinUI host sends Shift and Control before every pointer action.

**Tests.** `cascade.rs`: the keys on click, mouseDown, exit and change, both ways, and refused when untold; a
subform's enter refused and a write refused; `fullText` `123456` over a kept `1234`, `keyDown` 0 and 1, refused
when untold and on a click. `script_form.rs`: `Command::Keys` then clicks through the window; `keyDown` read as
the edit had it, not the commit. **Plant**: shift and modifier swapped, `fullText` as `change`, arrows never
recorded: the 3 tests fail; restored, all pass.

**Gates.** fmt `--check` and clippy `-D warnings --all-targets` (`pdf-model`, `pdf-formcalc`, `viewer-core`,
`viewer-ffi`): exit 0. nextest of the four: 3006 of 3008, the two failures `viewer-ffi`'s hard-coded entry-point
counts (361 to 362, 342 to 343), then `viewer-ffi` 77 of 77 with the C driver calling `quorra_window_keys`. `cargo
test -p conformance` after `git add -N`: exit 0. `state.sh --round 1959 docs`: exit 0. Tier 2: `-p pdf-model
--test script_corpus` under `bounded.sh --lock`, exit 0 (25 held documents on disk, 0 moved; 331 not on this
machine); the engine column not run, failing at HEAD. The C# is not compiled here; `header_and_interop_agree`
reads it.

**Left of the row**: the other events; an event's properties other than `script`; `commitKey` where no tab or
click took the focus (Enter on a choice list arrives as the same `CommitField` as a click on an item) and page
402's 0 (no host's Escape cancels a field's typing); the merge's format tests at `off`; a save's `cancelAction`
(ADR 2702); a transmitted submit's `postSubmit` with no answer; RFC 0011 step 5's events.
