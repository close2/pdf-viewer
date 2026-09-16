# 1147 — A restriction level per restriction, `off` by default, and a copy that can be asked about

The UI round: the first piece of the host UI, after the owner lifted `doc/todo/38`'s deferral.

**Measured first (trap 8).** Over the 90 763 files of `doc/pdf.js`, `doc/corpora/` and
`corpus-cache/`: 90 340 open without a password, 2 186 state an `/Encrypt` this reader reads, 4 open
as the owner. **1 539 withhold copying** (Table 22 bit 5), 1 655 annotating, 1 523 filling in,
2 025 modifying, 170 printing; **1 901 withhold copying or annotating.** Bit 5 was read by
`pdf-transform` and by nothing a person could press.

**Built.** `viewer_core::RestrictionPolicy` — one `RestrictionLevel` per
`pdf_model::restriction::Operation`, total over all six, every entry `Off` by default, carried by
`Command::Restrict` (a variant's shape changed rather than a message added, which is
`doc/ui-boundary.md`'s own preference and made five consumers fail to compile). `Command::Copy` and
`Event::Copied` are Table 22 bit 5 asked as an *operation*: a query could carry no level, because
refusing `Query::Selection` would refuse the highlight a drag draws and because a query raises no
event for `Ask` to wait on. `open::Held` is an enum now — an edit or a copy — both resolved before
they are held. The wire spells a policy as six level bytes in `RestrictionPolicy::OPERATIONS`'s
order, plus command kind 31 and event kind 20. The C ABI gained `quorra_restrict_operation`,
`quorra_copy`, `quorra_event_copied` and seven constants; `QUORRA_EVENT_KIND_COUNT` 20 → 21,
`QUORRA_ABI_VERSION` unmoved (no struct by value). `viewer_host::restrictions` parses
`--restrictions=copy:ask,annotate:on` for all three windows, and each sends `Command::Copy` on its
copy gesture and puts `Event::Copied` on its platform's clipboard.

**The default moved `On` → `Off`, the round's one behaviour change.** `CLAUDE.md` principle 3
decides it: restrictions "are low priority", "it shall always be possible to turn them off", "this
program is the reader's". Every other face already opened at `off`. §7.6.4.1's `shall` is one
`--restrictions=on` away rather than unread.

**Calibrated (trap 13)** in `crates/viewer-core/tests/restriction_levels.rs` on `bug1815476.pdf`,
whose `/P` of −1084 clears bits 5 and 6 and sets bit 9: at `Off` the copy proceeds, at `On` it is
refused by name with §7.6.4.2 in the note, at `Ask` the text is held until `Command::Answer` and a
`no` says nothing, at `Warn` the sentence follows the copy. One operation's level is shown to say
nothing about another's, both ways round; a document that restricts nothing proceeds silently at
all four levels; a copy of nothing asks nobody anything.

**Rows.** §6.3.2.1, §7.6.4.1, §7.6.4.2 and §12.5.3 gained the new tests, and §6.3.2.1 lost two
sentences that had stopped being true — bit 5 "not gated", and `Level::On` as the viewer's default.
No status moved: printing and assembling stay ungated because this crate performs neither.
Argument: ADR 1144. `doc/todo/38` and `doc/ui-boundary.md` are what-is.
