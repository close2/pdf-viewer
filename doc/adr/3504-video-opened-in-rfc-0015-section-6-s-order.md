# 3504 — Video opened, in RFC 0015 section 6's order, and the exclusion restated

Session 2200. Status: accepted. Context: `doc/questions/A946`, `doc/questions/A1006`;
`doc/scope-ratifications.md`'s clause 13 entry; RFC 0015 sections 6 and 12.

## Decision

A946 kept video closed until audio had shipped and its clock and sinks were measured in use, and
the owner then decided. A1006 is that decision, given after ADRs 3457 to 3468 re-measured audio.
`CLAUDE.md`'s clause 13 bullet and `doc/scope-ratifications.md` now place video decoded and
presented by this program in scope, in the confined media worker, built in RFC 0015 section 6's
fourteen rounds; interlaced H.264 stays on the hand-off to this machine's own player, said by name,
since no admitted decoder reads it. U3D and PRC stay excluded behind their triggers.

## Consequences

- No ledger row moves in this round. A row video unblocks leaves `out-of-scope` in the round that
  builds what it requires, never on the restatement.
- `tools/conformance/src/ledger.rs`'s `Exclusion::Multimedia` comment names A1006 beside A946; the
  word the ledger writes is unchanged.
