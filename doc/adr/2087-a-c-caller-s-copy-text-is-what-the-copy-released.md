# 2087 — A C caller's copy text is what the copy released

Session 1625, slot 3 of batch ninety-eight, a host round. Status: **accepted** and **built** in
`crates/viewer-ffi/src/session.rs` (`Session::copy_text`, the `copied` field, `handle`) and
`crates/viewer-ffi/src/events.rs::copied`, tested by `session.rs`'s
`the_copy_text_is_what_the_copy_released_at_the_readers_level`, `events.rs`'s
`a_copied_event_answers_as_the_hosts_one_decision_does` and the C program's `copy` lines. Context:
ADR 0519 (one choice of order for every host), ADR 1144 (the reader's four levels), ADR 1971 (the
soft hyphen's division undone at the copy), `doc/history/1621` "Found, not fixed" (2) and (3).
Clauses: ISO 32000-2 §7.6.4.2 Table 22 bit 5, §14.8.2.3, §14.8.2.5.1.

## 1. The defects

`quorra_selection_copy_text` answered from `Query::Selection` and `Query::LogicalSelection`, so a C
host was handed the text a copy carries without `Command::Copy`: Table 22's bit 5 — "[c]opy or
otherwise extract text and graphics from the document" — was never asked at the reader's level, and
the string kept the line break after a soft hyphen that the core's copy takes out
(`Seiten\u{ad}\nzahl.` against `Event::Copied`'s `Seiten\u{ad}zahl.`). And `Events::copied`
restated `viewer_host::copied`'s choice of order instead of calling it, and answered a copy of
nothing — which the shared decision refuses so that a clipboard is never emptied.

## 2. Decisions

1. **The readback is of the grant.** The session keeps what the last `Event::Copied` released,
   whichever call released it — `quorra_copy`, or `quorra_answer` after the ask level's question —
   put through `viewer_host::copied`, as it keeps a conversion (ADR 1954) and a press's sentence
   (ADR 2072). `copy_text` hands over that and nothing else. The entry point, its signature and its
   two-call idiom are unchanged; what changed is that a caller copies first. Sending `Command::Copy`
   from inside the readback was the alternative and it is wrong twice: the two-call idiom would ask
   the level twice, and an `Asking` or a `Warned` event would have nowhere to go.
2. **A copy asked again forgets the last one first.** `handle` clears the kept text when it sends
   `Command::Copy`, so a refusal, an unanswered or declined question, or an empty selection answers
   `QUORRA_NO_ANSWER` rather than the previous copy's text, which a caller would paste believing
   this copy released it.
3. **One decision, called.** `Events::copied` calls `viewer_host::copied` and returns the owned
   string; the C entry point copies it out as before.

## 3. Costs

A C caller that read `quorra_selection_copy_text` without `quorra_copy` now gets `QUORRA_NO_ANSWER`;
`crates/viewer-ffi/c/open_a_page.c` is the one caller in the tree and copies first. The .NET
declaration in `hosts/windows/Quorra.Interop/Native.Geometry.cs` is unchanged, because the
signature is.

`Query::LogicalSelection` is now asked by no host: the logical order reaches a C caller in the
copy's event, as it reaches the three windows, so `tools/state.sh hosts` counts the query as one a
C caller cannot ask, and that is the decision rather than a gap — asking it outside the copy is the
readback this record removes. The windows section's reading says so.
