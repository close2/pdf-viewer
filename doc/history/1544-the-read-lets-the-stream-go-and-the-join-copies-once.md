# 1544 — The read lets the stream go as it is read, and the join copies once

Slot 5 of batch eighty-six, 2026-10-14, a parsers round on the owner's switch to code improvement.
ADR 1924; no question. No ledger row moves: §8.4.2's stays `implemented`.
**Premise, held**: the pipeline kept every segment until its scope ended, for a later spine's
statement ranges and the fallback's concatenation. One fact the brief did not name: `Vec::split_off(0)`
copies, so the join copied each piece twice. **Hypothesis, two parts of three**: the read is no
slower and its peak is gone, but the peak does not fall under 1.1 GiB, and could not: ADR 1915 §3's
draw plateau is 1 238 MiB, and the peak is now in the draw, at 1 379.5 MiB.

**Built (ADR 1924).** A spine states copies of its kept statements, taken when their segment is
composed; a segment is let go once its piece has read it and its effect is composed, from the first
piece's end on; a fallback that finds one gone reads the stream again (`Outcome::Released`);
`DisplayList::split_off_commands(0)` hands the vector over. Not built: the lists handed to the drawer
in pieces (453 call sites read `&[Command]`), or a reservation written into directly (no piece's
count is known before it is read, and `forbid(unsafe_code)` holds).

**Figures** (HEAD, stream cut, both). Read, 24 threads: 429–456/392–419, 433–462/386–425,
388–431/406–476 ms; 12 threads: 438–506/439–480, 436–515/453–472, 435–482/441–500 ms. `wait4` peak,
huge pages off, 4 threads: 1 556, 1 439–1 440, 1 439–1 442 MiB; 24: 1 710–1 889, 1 576–1 683,
1 574–1 635 MiB. Massif, `--fair-sched=yes`: 1 442.5, 1 389.3, 1 389.3 MiB. Joins: 232.5, 232.7,
178.2 M instructions. Massif's default scheduler starves the calling thread and sees no release.

**Proof that nothing moved.** The witness's list digest is `07eb7445…68e3`, 4 408 405 commands, and
its frame one SHA-256, on all three arms; on the six corpus arms against `/home/AI/arms-1540/` (968
or 965 pages each) 0 digests and 0 mean errors differ; the measured export's three files are the
tree's, byte for byte. Plants caught: an empty re-read failed one divided-read test five runs of
five, and empty statement copies failed two.

**Unfinished.** The draw's peak is `pdf-render`'s: `mark_bounds` and `strip_inputs` hold two 84.1
MiB bound vectors at once. A page given up after its first piece now inflates its stream twice.
**Files.** `pdf-model/src/content/{divide.rs,divide/pipeline.rs}`, `pdf-render/src/display_list.rs`;
ADR 1924; `doc/state-of-play.md`, `doc/performance.md`, `doc/todo/69`.

**Gates.** `rustfmt --check` 0; `clippy -D warnings --all-targets -p pdf-model -p pdf-render` 0;
`nextest` 2 334 passed; `cargo test -p conformance`: one sibling failure (`viewer-qt`'s
`Host::arrivals`). Export walk (held 1 554 s) exit 0: `t2-corpus` 1, `t2-raster_golden` 2,
`t2-drawn_while_read` 2 passed, six arms exit 0. Clock hold (held 372 s, load 2.4–4.1): the read,
exit 1 for its render half (no `/usr/bin/time`), retaken by `wait4` under `--long`. Censuses exit
0 (213, 227, 468 s). Duration 9 500 s.
