# 1562 — A rendition's viability is judged and its clip handed to the host

Slot 5 of batch eighty-nine, 2026-10-15, a clause round. ADRs 1960, 1961; no question.

**Built.** `pdf_model::media::judge` is RFC 0009 §12 item 3's reading. It walks §13.2.3.3's tree and
applies §13.2.2's MH/BE rules and each table's own condition, judged for the hand-off, the one player
this program has, which is told nothing but the file (ADR 1960). `alternate` is §14.9.2.4's search.
`Rendition` is now read: Table 218 becomes `Action::Rendition`, and §12.6.4.14's precondition is
checked. The first viable clip goes to the host as `Event::Media`, after Table 286's /TF is asked as
`Operation::HandOverMedia` (`media:`, `off` by default). The confined wire and the C ABI carry it
(`QUORRA_EVENT_MEDIA` 28, `QUORRA_RESTRICTED_MEDIA` 8), and the windows say `handed_media` (ADR 1961).
Table 285's form XObject is on the excluded list, because the processor would be its player.

**Rows.** §13.2.2 `reported` → `implemented` ("In making these decisions, the viewer shall determine
the viability of the objects used."). §13.2.4.2 `reported` → `implemented` ("If the dictionary
contains unrecognised entries or entries with unrecognised values, it is considered non-viable, and
the interactive PDF processor shall not play the media"). §14.9.2.4 `reported` → `implemented` ("it
shall look for an exact (though caseinsensitive) match between the given language's identifier and
the language identifiers in the array"). §12.6.4.14 `out-of-scope` → `implemented`, on its
precondition and the /OP fallback. Under ADR 1599, §13.2 and §13.2.4 go to `out-of-scope`, and §14.9.2
and §14.9 to `implemented`. `doc/todo/65` and `state-of-play` follow.

**Premise.** The brief's `Media` level (`off`) and A254's (`ask`) are two: Table 286's is built, and
starting a player is owed, since no window starts one (ADR 1961 §1). Hypothesis held: nothing decoded.

**Drive** (its outputs in the round's scratch): `multimedia_annotations.pdf`, and a copy with /TF `TEMPNEVER`.
In quorra, quorra-gtk and quorra-qt, `media:off` prints viability and the hand-off. `media:on` refuses
the copy with Table 286's sentence and hands over the original (`TEMPACCESS`, not encrypted). `warn`
hands over and then warns. quorra-confined does not act on the click, so it was not driven. **Nothing
drawn moved**: HEAD plus my `pdf-model` paths (`r1562-media`) against HEAD's six arms
by page, has 5 799 digests and 0 differ.

**Gates.** `rustfmt --check` on my files: 0. Clippy `-D warnings --all-targets --keep-going`:
`pdf-model`, `viewer-core`, `viewer-confined`, `pdf-vfs` and `pdf-transform`'s lib are 0, apart from
neighbours' `page_tree_nodes.rs`, `tests/archiving.rs` and `quorra-transform.rs`. `viewer-host`'s lib
fails on a neighbour's `archiving.rs`, so `viewer-ffi`, `-ui`, `-gtk` and `-qt` were not linted.
Nextest 0: `pdf-model` lib + `actions` + `save_round_trip` (664), `viewer-core` + `pdf-vfs` (472),
`viewer-confined` + `viewer-ffi` + `viewer-host` (409). A /TF plant failed 2 of the 5 `renditions`
tests. `cargo test -p conformance` 0. `tools/state.sh docs` 0. `t3-actions` 0. Six arms 0, in one
hold of 1 746 s. About 6 500 s.
