# Round 2182 — audit 2180's fixes: an inherited CMap, a symbolic font's ignored /Encoding, a Type 3 /LastChar

A fix round in the `gwg` worktree for audit 2180 (§9.5 to §9.10): its three B findings and the
three open D findings. ADR 3429.

## What changed

- **B01** `cmap.rs`: `CMap::absorb` lays each length's mappings over the inherited ones through
  `Mapping::override_with`, which removes an inherited single a referencing range restates —
  the one pairing `Mapping::get`'s singles-first order lost. The notdef mappings take the same
  path. A unit test probes all four pairings for both kinds.
- **B02, D04** `truetype.rs`: a symbolic font's `/Encoding` feeds no step of §9.6.5.4. Below
  the code offered as a character, its name is carried to the subtables or `post` as a
  recovery of the processor's choosing, recorded in `reader_chosen`; the names a symbolic font
  keeps for §9.10.2 are the ones that recovery used (ADR 3429). The dead
  `UnsupportedEncoding` arm is gone: a nonsymbolic font propagates, a symbolic one loses only
  the recovery.
- **B03, D02** `type3.rs`: `/Widths` is cut at `/LastChar` when read; an absent `/LastChar`
  leaves the array's length, as `stated_widths` does. Three history comments now state the
  current reason and cite ADR 1059.
- **D03** `composite.rs`: `collection_table` writes down why the descendant's `/CIDSystemInfo`
  stands for the `CMap`'s (§9.7.3), and loses a stray doc line from another function.

**Outcomes.** Golden: held 974, moved 0 (no tracked first page has a symbolic `TrueType` font
whose `/Differences` decided a glyph). Archive: `pdf-transform --test archive` filtered to
`truetype|symbolic|reader`, 73 passed. Manifest: the three 2180 entries removed from
`expected-failures-current.json` (now empty); 2180's B01–B03, D02–D04 marked `fixed`.

**Ledger.** 9.7.5.3, 9.7.6.2, 9.6.5.4, 9.10.2, 9.6.4 re-read: `implemented` before and after,
each note given the reading it was contradicted on and the tests that hold it.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets -p pdf-font -p pdf-model -p pdf-archive
-p viewer-qt` exit 0 (101 first: `doc_markdown` in quoted text). `pdf-sandbox` 0. nextest `--lib
--bins --tests` of the four (`xvfb-run -a`): 3814 passed, 27 skipped, exit 0; `audit-diff.py`
"failed 0, expected 3", the three gone. Golden walk exit 0, moved 0. rustdoc `-D warnings
--document-private-items -p pdf-font -p pdf-model` exit 0. `cargo test -p conformance` exit 0
(101 first: this record's own length and Gates paragraph). Fuzz not owed: no public type changed.

**Habit.** A recovery from an entry the clause ignores goes below the clause's own fallback,
recorded as the reader's choice — never above it, where it decides silently.
