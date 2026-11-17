# 2937 — A C host is handed a policy's referenced lists

Session 2061, a signatures round (`doc/todo/77` section 2 item 4). Status: implemented in the ABI;
the WinUI window owes its half. Rows: §12.8.3.4.4. Context: ADRs 0250, 0576, 2879, 2886, 2932, 2936.

## Context

ADR 2879's C host supplies its trusted lists as bytes through `quorra_trusted_lists` and fetches
them itself; ADR 2886's `quorra_window` fetches inside `QUORRA_ACT_TRUSTED_LISTS`. ADR 2936 put a
policy's references behind a question, which `tests/every_query_reaches_the_abi.rs` requires to
reach the ABI.

## Decision

1. **Two entry points, C's two-call idiom**: `quorra_trusted_list_references(viewer, &count)` and
   `quorra_trusted_list_reference(viewer, index, out, cap, &needed)`, `QUORRA_NO_ANSWER` where
   nothing is focused. Added entry points move neither `QUORRA_ABI_VERSION` nor the event count.
2. **A `quorra_window` caller owes nothing**: the window asks the core inside
   `QUORRA_ACT_TRUSTED_LISTS`, through the shared code ADR 2936 extended.
3. **A session caller that fetches its own lists** puts these locations to the reader at their
   network level, naming each, fetches them beside the Commission's list, and supplies what it kept
   through `quorra_trusted_lists`, where the location's last path segment names the file (ADR 2842).
   Authenticating a referenced list is `refresh_with`'s, which the ABI does not expose: such a caller
   keeps only what it authenticates, as for every list it supplies.

## What the WinUI window owes

Nothing for its Shift + T, which is the C window's act. If it ever fetches lists itself rather than
through `quorra_window`, it asks `quorra_trusted_list_references` first and names the locations in
its question. The C# declarations are in `Native.Queries.cs`; no Windows build ran here.

## Tests

`open_a_page.c` asks both entry points and prints the count, which the Rust test reads as 0 for the
unsigned application note; `unsafe_position.rs` counts 382 entry points.
