Status: complete
Given: 2026-10-26, in conversation — transcribed by the round
Owes: the rule — by ADR 2372 section 3: the orchestrator clears after the merge and
`tools/batch.sh close`, never with a round in flight, after a handover holding the owner's
unrecorded words verbatim, the machine's reds by test name, the next candidates with their
premises, the numbers assigned, and "nothing runs"; the next batch opens in a fresh session
reading `doc/orchestrator-handover.md`, the memory index and that handover. The agent-type tool
allowlist is measured first: one batch's rounds on the allowlisted type, the census's
first-request size compared, then adopted or not on that number (acting round)

> Otherwise I agree with all recommendations.  (only departures: XFA and Q632, which I haven't
> fully understood yet)

Reading: one message answered six questions together and named its two departures; this one
adopts Q635's recommendation as presented, quoted and attributed: *"Yes to 1, by ADR 2372
section 3 … **Yes to 2, measured first**."* The question's source is the owner's own request of
2026-10-26 — "we might even should clear this context between batches. Are there reasons not
to?" — so the answer ratifies what the owner proposed, with the two guardrails the measurement
added.

What the answer settles:

- **The clear is safe only when nothing runs**: a report reaches the session that launched its
  round, so the clear comes after the merge and the close, never with a round in flight.
- **What survives is what the handover writes**: the owner's mid-batch words not yet in memory,
  verbatim; the machine's reds by test name; the next candidates with their premises; the
  numbers assigned. The judgement a session holds — which reds are the machine's, what a sibling
  touched — is written down rather than lost.
- **The saving is real and bounded**: ~70 % fewer tokens late in a cycle (~$4.6 a batch of its
  $12), and no compaction at ~970k written under pressure at a turn nobody chose — while the
  rounds' own context (ADR 2372 rule 1, about 20 %) remains the larger term.
- **The allowlist is a configuration change no round may make**, so it rides on a measurement:
  one batch on the allowlisted agent type, the fixed 39k first request (19 % of a round's reads)
  compared before the change is adopted.