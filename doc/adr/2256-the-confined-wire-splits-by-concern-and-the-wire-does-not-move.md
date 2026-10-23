# 2256 — The confined wire splits by concern, and the wire does not move

Session 1711, the standing code-quality slot of batch 109. Status: implemented; the bytes are unchanged.

`crates/viewer-confined/src/protocol.rs` was the largest source file in the tree: the greeting, the
frame kinds, two refusals, the writer and reader, four message families (each an encoder, a decoder
and a table of discriminants), the page-crossing tables, the measurement codecs and about 3 000
lines of tests. Rounds 1670–1696 had already moved the larger payloads (`panels`, `display_list`,
`windowed`) and two small codec families out; this round moves the rest by concern.

**The shape.** The root keeps the module documentation, the greeting (`MAGIC`, which
`tools/conformance/tests/greetings.rs` reads from this file), the frame kinds and payload
discriminants, and the `pub(crate)` re-exports a sibling file calls. Under it, one module each:
`wire` (`Writer`, `Reader`, `read_quads`), `frame` (header, handshake), `error` (`Uncarried`,
`ProtocolError`), `command`, `event`, `query`, `answer` (discriminants, encode, decode),
`view_fields` and `edit` (the fields a command shares), `marks` and `payload` (what a page crosses
as and the lists a host holds), `measurement` (§12.9 and the located point). The tests are
`protocol/tests/`, five files: commands, events and queries, panel answers, frames, refusals.
Each module imports from the sibling that defines a name, not through the root.

**What did not move.** Every byte of the wire: the greeting `PDFVCF18`, every discriminant, every
field order and every refusal sentence. Nothing in `display_list.rs` or `panels.rs` changed but two
call paths (`super::frame::as_u64`, `super::wire::read_quads`).

**Visibility.** The split needed `pub(super)` on what a sibling calls; a pass that removed each
one in turn and compiled after each narrowed those nothing read. The discriminant constants in the
`*_kind` modules are `pub(in crate::protocol)` because the tests read them (before the split
`pub(super)` meant the same). `Writer`'s and `Reader`'s methods are `pub(super)` because every
codec calls them.

**Proof.** The multiset of tokens of the non-comment lines before and after matches once the
imports, visibility prefixes and module declarations are removed from both sides: the differences
are the closing `mod tests {` and ten trailing commas rustfmt adds when a dedented test call no
longer fits a line. The script is `scratchpad/r1711/proof.py`. 31 test functions before, 31 after.

**Ledger.** The one row naming the file (§12.6.4.3, Remote Go-To) names `protocol/command.rs` and
`protocol/event.rs`, where `Purpose::RemoteDocument` and the supplied file cross.

**The flags gate.** `tools/conformance/src/flags.rs` treated a test module as "not the program" only
when it was inline; a `#[cfg(test)] mod tests;` whose files live beside the parent now counts the
same, so a flag named in a test fixture is not read as a message a user is shown.
