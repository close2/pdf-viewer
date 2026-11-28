# 3883 — The rich media rows take the states their clauses give them

Session 2295. Status: **accepted**. Ledger: ISO 32000-2 §13.7 to §13.7.2.3.6.3. Builds on ADRs
3820, 3854, 3855, 3865 to 3867, 3881, 3882; `doc/todo/80` section 1.

## Context

ADR 3820 kept every rich media row `out-of-scope` until a round built it. `doc/todo/80` section
1's five steps are now built or read: settings and activation, presentation, configurations and
instances, views and snapshots (the snapshot said, not displayed), and the animation (read and set
aside for a clip). The ratification makes rich media follow its parts, so a row whose `Sound` and
`Video` half is built leaves the exclusion; its `3D` half stays on the clause 13 entry, named in
the note, as §13.6.2's row carries the artwork's rendering beside its built appearance (ADR 1946).

## Decision

1. **`implemented`**: 13.7.1 (states no requirement), 13.7.2.2 (heading), 13.7.2.2.1 to 13.7.2.2.3,
   13.7.2.2.5, 13.7.2.2.6, 13.7.2.3.3 and 13.7.2.3.4. Each note names its built sentences, the code
   and tests, and the `3D` configuration and `/Scripts` as the entry's.
2. **`reported`**: 13.7.2.3.5. Table 344's "shall be displayed in place of the current artwork" is
   not built; it is said on every activation invoking a view with a snapshot, and ADR 3881 states
   what moves it.
3. **`out-of-scope`, restated**: 13.7.2.2.4, whose subject the clause places in "3D model content"
   (ADR 3882), and 13.7.2.3.6 to 13.7.2.3.6.3, the run time's state protocol, stated only as ISO
   21757-1's `StateEvent`.
4. **The headings 13.7, 13.7.2 and 13.7.2.3 are `partial`**, derived from their leaves.

## Consequences

The rows now say what the tree does rather than what a plan intends. One row owes: the snapshot,
whose display waits for the players to tell the core when a clip leaves their hands.
