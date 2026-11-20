# 3265 — A `/CIDSet` marking a CID the program does not hold fails ISO 19005-2 section 6.2.11.4.2

Session 2141, an archive round in the `gwg` worktree. Status: **accepted** and built. Completes
ADR 3255's "What veraPDF does differently". Context: ADRs 3239, 3255;
`crates/pdf-archive/src/table/fonts.rs` (`cidset_lists_every_cid_in_the_program`,
`marked_but_absent`). Clauses: ISO 32000-2 §9.8.3.1 Table 124; ISO 19005-2 section 6.2.11.4.2.

## Context

Section 6.2.11.4.2 asks a `/CIDSet` to identify all CIDs present in the font program (cited,
not quoted: `doc/third-party-data.md`), and Table 124 calls the entry "a stream identifying which
CIDs are present in the CIDFont file". The check asked only one direction: a CID present and not
marked. veraPDF 1.30.3 also fails a set marking a CID the program lacks (ADR 3255).

## Decision

A set identifies the CIDs present; a bit set for a CID the program does not hold identifies an
absent CID as present, so it fails the same requirement, with its own finding ("marks N CIDs the
font program does not hold, including CID c"). Present is ADR 3255's reading: the identity map's
glyph indices, a map stream's reachable CIDs, a CID-keyed CFF's charset. A bit past CID 65 535 is
not read: no two-byte code selects it. Both directions at one site are two findings. The
requirement's `asks` line names both.

## Consequences

The converter's remedy is unchanged and already lossless: `DescriptorSetRemoved` takes the
`/CIDSet` out (Table 124 makes it optional), so a source failing the new direction converts as one
failing the old. Test: `a_cidset_marking_a_cid_the_program_does_not_hold_fails`.
