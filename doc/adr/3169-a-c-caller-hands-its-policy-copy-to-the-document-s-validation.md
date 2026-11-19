# 3169 — A C caller hands its fetched policy copy to the document's validation

Session 2117, a host round in the `gwg` worktree. Status: accepted and built. Row: §12.8.3.4.4.
Context: ADRs 1738, 1753, 2624, 2725, 3162.

## Context

ADR 3162 left §12.8.3.4.4 one piece short: `quorra_event_policy_bind` binds a copy of a signature
policy's document that a C caller fetched itself and says what it comes to, and nothing in
`viewer-ffi` handed that copy to the document's validation as `viewer_core::Command::PolicyCopy`,
which the three windows send after their own fetch (`viewer_host::submit::Returned::follow_up`,
ADR 2624). The copy was compared, and its constraints were never enforced.

## Decision

`quorra_policy_copy(viewer, published, index, policy, bytes, len, events)` sends the command.

- **The policy is named by the event that listed it**, as `quorra_event_policy_bind` names it:
  `published` is the `QUORRA_EVENT_SIGNATURE_POLICIES_PUBLISHED` list, read and not consumed, and
  `Events::published` takes the document and the `PublishedPolicy` from it. A caller never rebuilds
  a policy value from strings, so the copy reaches the core with the identifier and commitment the
  signature stated, and the core binds it from those and never from the caller's word.
- **Errors are the neighbours'**: a null handle is `QUORRA_NULL_ARGUMENT`, as is a null `bytes`
  with a nonzero `len` (a null `bytes` with a zero `len` is an empty copy, as in the bind); an event
  or a policy the list does not hold is `QUORRA_OUT_OF_RANGE` and an event of another kind
  `QUORRA_WRONG_KIND`, and nothing is sent for any of them.
- **The two calls stay apart.** The bind says whether the copy opens beside the document; this
  call enforces it. A caller may make either without the other, as a window's fetch makes both.
- The bind's closing sentence (`viewer_host::policy`'s `Onward::ByTheCaller`) now says the
  constraints are enforced from the copy once it is handed on, in place of saying the interface has
  no call to do so.

The C header, the WinUI host's P/Invoke declaration (`Native.ToolkitCommands.cs`) and
`unsafe_position.rs`'s count (383 entry points, 361 `unsafe`) carry it. The test is a C program,
`c/hand_on_a_policy.c`, run by `a_c_program_drives_the_abi.rs` on the windows' shared fixture
(`signed_under_a_policy.rs`): before the copy the document references no trusted list, after it
the one its bound policy's trust point names, which only an enforced copy yields (ETSI TS 119
172-2 clause 4.22.2.1 b), ADR 2936). A plant that bound the copy and did not send the command left
the second reading at zero and failed the test.
