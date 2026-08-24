# 717 — The floor that was a widget list, and the half of a flag nobody read

Date: 2026-08-24. ADR [0596](../adr/0596-the-floor-that-was-a-widget-list.md).

Touched: `crates/viewer-gtk/src/{controls.rs,host.rs}`; `crates/viewer-host/src/form.rs`;
`crates/viewer-host/tests/host_mappings.rs`; `crates/viewer-qt/cpp/window.cpp`;
`crates/viewer-ui/src/chrome.rs`; `crates/viewer-ui/src/bin/pdf-viewer.rs`;
`crates/viewer-ui/src/bin/pdf-viewer/{typing.rs,window.rs,overlays.rs,app.rs}`;
`crates/viewer-ui/tests/panel.rs`; `doc/conformance/ledger.toml` (§12.7.5.4);
`doc/todo/30-a-native-host.md`, `doc/state-of-play.md`, `doc/traps/the-interactive-loop.md`,
`doc/HANDOVER.md`.

The **seventh** round on the project owner's *"even though low priority, I think we should start
investing time into the UI (and its API for the native versions)"*, taking **item 7** of ADR 0509's
ordering — the last one on that list and the one it called "a real toolkit floor".

`viewer-core` was not touched. No message was added and no variant changed shape — the twelfth
consecutive round since the six-hundred-and-seventh in which that has been true.

## It was not a floor, and the check cost one screenshot

ADR 0509 §3 recorded the block and attached ADR 0508's rule to it: *call the API before writing that
something is blocked on it.* Every sentence of the block is true about widgets — `GtkDropDown` has
no entry, `GtkComboBoxText` is deprecated in the release this crate binds, and this workspace turns
warnings into errors. There is no GTK 4 widget that is an editable combo box.

Table 233 bit 19 does not ask for a widget. It asks for "an editable text box as well as a
drop-down list", which is a `gtk4::Entry` beside a `gtk4::MenuButton` over a `gtk4::ListBox`, in one
box with GTK's own `linked` style class. **The `v4_10` feature floor did not move**, which is the
half of ADR 0508 that mattered: what a feature floor costs is a runtime requirement on everybody who
installs this program.

Driven under `Xvfb` on `doc/pdf.js/test/pdfs/issue17492.pdf`, whose first page states an editable
combo box (`country`, 28 options) beside a non-editable one (`jobDescription`) — so one window shows
both halves of the flag at once. The composed control lists Table 234's `/Opt` in the array's own
order (Table 233 bit 20), picking *Germany* wrote `/V` and the entry showed it back, and typing
*Atlantis* into it held a value none of the 28 options states, which is exactly what §12.7.5.4 says
the flag admits.

## The half nobody had read, and it was the host that is ahead

The bit's second clause is a `shall` in the other direction: **if clear, it shall include only a
drop-down list.** `viewer-ui` broke it for the whole of its life.

`Answer::Field` answers a combo box with characters whether or not the flag is set, and that is
correct — the value *is* a text string and §12.7.4.3 lays it out. The host read *has a text value* as
*takes typed characters*, so a person could put the caret in a drop-down stating Red and Blue and
type *Purple*. This is Table 229 bit 26's shape one flag over (ADR 0346): the set direction reported
as unimplemented while the clear direction was broken in silence, because a flag reads as a
permission and half of these are prohibitions.

`viewer_host::form::ControlKind::takes_typed_characters` is the one statement now, exhaustive over
the enumeration so a ninth control kind fails to compile rather than falling into an arm. The window
refuses by name, quoting the clause.

## The tier-2 host could not choose an option at all

Reading the clause for bit 19 found the larger gap: `Entered::Chosen` — the variant `Edit::SetField`
grew in the four-hundred-and-twelfth session so that §12.7.5.4 could say which options are selected
— occurred **nowhere in `viewer-ui`**. It sends no `Command::Delegate`, so no `GtkDropDown` was ever
placed over its page: a list box drew its options and no press could select one, and a combo box
could only be given a value by typing a label, which is the violation above. So refusing the
keyboard without giving that host the control would have closed a clause violation by removing a
capability.

`viewer_ui::chrome::ChoiceList` is the drop-down drawn rather than placed. One layout answers both
*where the rows are* and *which option a press landed on*, because two derivations is how a control
comes to show one row and act on another and no gate in this tree rasterises chrome. It opens at
Table 234's `/TI` for a list box and at the value for a drop-down, on the entry's own wording ("[f]or
scrollable list boxes"), and the options past the twelfth are counted rather than dropped.

Under `Xvfb`, on the same file: the country list opened with *Spain* marked and *16 more* counted,
picking *Poland* wrote it and the page's own appearance redrew with it; the non-editable combo
printed the refusal and opened its three options with no caret in the value; and the multi-select
list box took two rows and stayed open, which Table 233 bit 22 is why.

**One thing only the screen said**, and it is the file's rather than ours: `databases` states its
`/V` on the widget in a way `pdf-model` resolves to no selection at all — `headless.rs` has asserted
"the field states no `/V`" since ADR 0235 — while the stream the *file* supplies highlights three
rows. Choosing in that list therefore regenerates the appearance and the file's blue bands go, which
is ADR 0407's decision working: the clause states the options and states no highlight, so the
selection is reported and the mark is the host's.

## What was run

`fmt`, `clippy --workspace --all-targets` under `-D warnings`, `nextest --workspace`, the workspace
doctests, `check` over the fuzz targets, and `cargo test -p conformance` — the core plus the
conformance gate, which is `doc/todo/02` §2's row for a change to three host crates and the ledger.
§5's binaries were rebuilt and installed.

Nothing here can change a pixel a corpus gate rasterises: no `pdf-*` crate, no rasteriser, and
`viewer-core` untouched.

Both new tests were run against injected defects before being believed (trap 13):
`takes_typed_characters` made to answer true for every combo box, and the drawn rows offset from the
rows the layout reports. The quotation gate caught two things worth naming — a `§12.7.3.2` that is
not a clause of this standard, and a blockquote of Table 233 bit 19 that `doc/md`'s extraction
hyphenates as *drop- down*, so the load-bearing sentence is prose with the fragments quoted inline,
which is what the rest of this tree does with that table.

**One thing the run itself said, and it is `doc/todo/02` §2's loaded-machine rule in a shape that
paragraph does not cover.** With four rounds building at once — load average 331 —
`pdf-model::outlines::an_outline_resolves_against_the_page_tree_once` failed, and it is a *ratio*
test written so that a slow machine cannot fail it: it times a search it performs itself and
compares. A ratio survives a slow machine and does not survive a scheduler that stalls one of the
two phases and not the other. It passed alone in the same tree and passed in the whole sequence once
the machine was quiet. §2's sentence is about a gate that spawns a reference renderer; this is one
process, and the rule generalises — **any gate that times anything is a measurement of the machine
too.**

`tools/state.sh windows` is unchanged by this round, which is right: `viewer-ui` already asked
`Query::Fields` and `Query::FieldAt`, and what it gained is an answer it was not using.

## What is left, named rather than left silent

The list has no keyboard in `viewer-ui`. Up, Down and Enter would be this host's convention and no
clause states one; Escape closes it because a control that can be opened must be closable without
picking something.

And the two items the round before this one named are both still open: `AccessibilityNode::lines`
does not cross the C ABI, and `tools/state.sh windows`' eleven unreached queries are still an
uninterpreted list. This round took item 7 instead because it is what `doc/todo/30`'s ordering says
is next and because the first job that ordering set — establish whether the floor is real — turned
out to answer *no* and to have a clause violation behind it.

Nothing is queued for the owner's measurement loop: every number here is `Xvfb`, a corpus document
and a rasterised display list.
