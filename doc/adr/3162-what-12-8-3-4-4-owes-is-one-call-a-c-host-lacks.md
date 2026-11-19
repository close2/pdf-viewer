# 3162 — What §12.8.3.4.4 owes is one call a C host lacks; the rest is limits and the reader's choice

Session 2115, a clause round in the `lane3` worktree. Status: accepted. Row: §12.8.3.4.4 (stays
`partial`); its aggregates §12.8.3.4, §12.8.3 and §12.8 stay derived (ADR 1599). Context: ADRs 1035,
1599, 1728, 1753, 2624, 2711, 2725, 2782, 2932, 2936, 2937, 3158, 3161; `doc/questions/A98`.

## The residue, re-read

The row's note named two pieces of work (ADR 2782, re-stated by ADR 3158): a TR 102 272 value whose
type's equality rule was not held, and a trusted-list anchor whose list nobody supplied. ADR 2725's
table names a third, "a C caller's fetched copy", which the note's residue sentence had dropped.

| piece | now | kind |
|---|---|---|
| the six X.520 rules and `id-at-role` | built, and `role` has no rule (ADR 3161) | done |
| a clear-text password against an encrypted one | no Recommendation defines the encryption | limit (ADR 3161) |
| a value outside its type's syntax, a `TeletexString`, a BER bit or octet string | X.501 clause 8.9.2.1 (a)(3), X.520 clause 7.1 | limit (ADRs 3158, 3161) |
| a trusted list the policy references | fetched by every window and given to a C host (ADRs 2882, 2886, 2932, 2936, 2937) | done |
| a list the reader declines to fetch | A98's `Submissions` level, `ask` by default | the reader's choice |
| human-readable policies, revocation `other`, an undefined signed assertion, a TS 119 172 value constraint, clause 6.11's extensions | ADRs 2711, 2725 | limit |
| clause 6.5.2's verifier rules, rules scoped to generation or augmentation | addressed to a signer or an augmenter, not a validator (ADR 2711) | outside validation |
| a policy in a syntax no specification this program holds defines | as human-readable: its constraints are in a language nobody published | limit |
| a policy copy a C caller fetched | `quorra_event_policy_bind` binds it and says nothing was enforced; no entry point hands it on as `Command::PolicyCopy` (ADRs 1753, 2624) | **work** |

An anchor left undetermined because the reader said no to a fetch is not an unexecuted requirement:
the program does what TS 119 172-2 clause 4.22.2.1 asks the moment it is allowed to. A C host's copy
is different: the host fetched it, and the program cannot enforce it because its interface has no
call to receive it, while the three windows' copies are enforced (ADR 2624).

## Decision

The row stays `partial` on that one piece (ADR 1035), and its note says so with what answers it: an
entry point in `viewer-ffi` that hands a bound copy to the document's validation as
`viewer_core::Command::PolicyCopy`, with its C and C# declarations and the entry-point count. It is a
host-lane build, not a signature-reading one, and it was not taken in this round, whose budget the
six rules spent. No question is asked: nothing in it needs the owner's word.
