# Round 2117 — a C caller's policy copy is enforced, and §12.8 is `implemented`

A host round in the `gwg` worktree on §12.8.3.4.4's last piece (ADR 3162).

## Built

- `quorra_policy_copy` (ADR 3169): a C caller's fetched copy of a signature policy's document, named
  by the `SIGNATURE_POLICIES_PUBLISHED` event that listed it, goes to the document's validation as
  `viewer_core::Command::PolicyCopy`, as the windows' copies do. Header, `Native.ToolkitCommands.cs`
  and `unsafe_position.rs` (383 entry points, 361 `unsafe`) carry it; `quorra_event_policy_bind`'s
  sentence now says the copy is enforced once handed on.
- `c/hand_on_a_policy.c`, run by `a_c_program_drives_the_abi.rs` over the windows' shared fixture:
  no trusted-list reference before the copy, the bound policy's one after it, and the two refusals.
- RFC 4518's whole preparation for RFC 5280 section 7.1's name comparison (ADR 3170,
  `trust/names/string_prep.rs`): ADR 2725 named a non-ASCII distinguished-name value as work, ADR
  2775 dropped it from the residue, and the code still left it undecided.

## Ledger

§12.8.3.4.4 `partial` to `implemented`; §12.8.3.4, §12.8.3 and §12.8 follow (ADRs 1599, 3171).
`doc/todo/65` takes the row out of bucket 2 and empties the aggregate list.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 over `pdf-signature`, `viewer-ffi`,
`viewer-host`; `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` of the three, 1090 run, 1090
passed after `header_and_library_agree`'s second count of entry points moved to 383, no difference
from `expected-failures-current.json`; rustdoc `-D warnings --document-private-items` 0;
`cargo test -p conformance` 0. The plant (the entry point sending nothing) failed the C test with
"after: 0 reference(s)".

## Trap

Round 2115's trap held a second time: ADR 3162's table was written from the note, and the note had
lost a row of ADR 2725's table two decisions earlier. Read every table back to the first one.
