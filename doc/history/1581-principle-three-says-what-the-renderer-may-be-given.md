# 1581 — Principle 3 says what the renderer may be given, and a census of nothing says so

Slot 5 of batch ninety-two, 2026-10-19, a docs round. No ADR (1998, 1999 unused), question or row.

**Premise.** Held: `grep -c A24 CLAUDE.md` was 0; `A24` reads "yes, take the suggested wording".
**Did not hold**: the census's `--since 2026-10-19T12:40Z` is later than every request batch
ninety-one sent — its walks in `/home/AI/heavy-walk.log` run 08:03–11:17 local, and its merge is
09:18Z — so the brief's command would have counted nothing and printed a page of zeros.
**Hypothesis.** Not tested: the transcripts were not read (Unfinished).
**Built.** (1) `CLAUDE.md` principle 3's sandbox bullet carries `Q24`'s proposed replacement
verbatim, its line breaks kept, with the owner's answer quoted and `A24` and `doc/todo/59` cited.
(2) `tools/cache-census.py` refuses a census that counted no request — exit 2, one line naming the
`--since` and the newest stamp — where it printed zeros as if no gap lost the cache (trap 13);
planted with the brief's `--since` on a synthetic transcript and on an empty file. (3) The
navigational re-read against ADRs 1928–1989: `doc/state-of-play.md`'s confined decoder (it admits a
page past the bound as marks or as its worker's window, ADRs 1944, 1986), the eight operations now
nine (`HandOverMedia` reads Table 286's `/TF`, ADR 1961), `quorra-confined` no longer "performs no
restricted operation" (its click reaches a check box and a rendition, ADR 1987), ADR 1868's band
narrowed to a page drawn whole (ADR 1930), the Control bindings named by `ctrl_meaning` rather than
counted "four" (ADR 1985); a one-line title (ADR 1973) and `xfa` absent (ADR 1989) added;
`doc/HANDOVER.md`'s conversion row names `doc/todo/72`.

**What the waits looked like, from the lock's log alone.** Rounds 1571–1576 ran 87 locked runs; 60
queued and held past 240 s, 53 past 300 s — each a lost cache unless waited for through
`tools/wait.sh`. No record of 1571–1576 names `wait.sh`; whether they used it is the transcripts'.

**Proof that nothing moved.** No crate touched; the census's change runs only when nothing was
counted. Files: `CLAUDE.md`, `doc/state-of-play.md`, `doc/HANDOVER.md` line 52 (65 is a sibling's),
`tools/cache-census.py`, this record.

**Gates.** `cargo test -p conformance --no-fail-fast` exit 0, 57 of 57 targets (an earlier run's
one failure was a sibling's `tools/state.sh:259`, since mended). `--bin
quotations` exit 0, `--bin pointers` exit 0 (13 absent, 4 slot 1's unwritten ADR 1990, none mine).
`tools/state.sh --round 1581 docs` exit 0, 54 s.

**Unfinished.** The census: listing the task links was refused by the permission classifier, twice,
so no transcript was read and ADR 1976's successor is unwritten. The command: `python3 -I -B
tools/cache-census.py --since 2026-10-19T05:53Z --buckets <batch 91's round transcripts>`, against
ADR 1976's 85 %. `quorra-confined.rs`'s `Asking` comment ("no operation of its can be restricted")
is stale the same way and is not this slot's file.
