# 2061 — A policy's referenced lists reach the refresh through every window

Session 2061, a signatures round in the gwg worktree (`doc/questions/A800` item 3, `doc/todo/77` section 2
item 4; §12.8.3.4.4). ADRs 2936 and 2937; no question.

**The slice.** What ADR 2932 left: the references a policy's `TAsInTrustedList`s make, from the core
to each window's question and refresh, and across the C ABI.

**The premise.** Held: no caller passed references to `refresh_with`; `trusted_lists` in the shared
window code knew only the directory. ADR numbers checked: 2932, 2886, 1728, 2882, 2874, 2879 are as
the brief named them. No owner's word: ADR 1728 already asks before fetching a location a document
names, at A98's `Submissions` level, so Q889 was not written.

**Built.** `PolicyDocument::trusted_list_references` on TS 119 172-2 and -3 documents (one walk over
the shared rules; clauses 4.24 to 4.26's trust trees). `Enforcement::Enforced` carries them, so only
a bound copy's count. `Query::TrustedListReferences` answers the focused document's, and crosses
`viewer-confined` as kind 41. The shared `requests::trusted_lists` (GTK, Qt, the C window) asks it,
the question names each location and clause 4.22.2.1 b), `Asked::TrustedLists` keeps what it named,
and `Refresher` hands them to `refresh_with`; `quorra`'s own dispatch does the same. The C ABI gains
`quorra_trusted_list_references` and `quorra_trusted_list_reference`, in the header, the C driver and
`Native.Queries.cs`. An event was refused for a question: a copy fetched later adds references.

**Tests.** pdf-signature: references of two rules, in order and once, and none. viewer-core: an empty
list, then the reference once a bound copy is handed in; a copy with one octet added answers none.
viewer-confined: the query round-trips, its answer too. viewer-host: the question names the location
only when given one; a refresh asks for it last, keeps `AT`'s list there and hands it to the core.
The C driver prints the count, 0. **Plant**: the XML document's references dropped and the
`Refresher` given none: the core and host tests fail; reversed, they pass.

**Gates.** Over `pdf-signature`, `viewer-core`, `viewer-confined`, `viewer-host`, `viewer-gtk`,
`viewer-qt`, `viewer-ffi`, `viewer-ui`: fmt exit 0; clippy `-D warnings --all-targets` exit 0;
nextest under `xvfb-run -a` 1974 of 1976, exit 100 on two counts (41 queries, 382 entry points),
both fixed and passing in the plant run (82 of 84, the two planted failing); rustdoc `-D warnings
--document-private-items` exit 101 on a private link, then 0; `cd fuzz && cargo check --bins` exit 0;
conformance after `git add -N` exit 0 (474 passed).

**Left.** The WinUI window owes nothing for Shift + T, which is the C window's act; it owes the
references only if it fetches lists outside `quorra_window` (ADR 2937). No window-level test drives
a signed document through Shift + T. §12.8.3.4.4 stays `partial`.
