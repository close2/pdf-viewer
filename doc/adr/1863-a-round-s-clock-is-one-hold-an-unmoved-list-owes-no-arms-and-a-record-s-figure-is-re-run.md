# 1863 — A round's clock is one hold, a list shown unmoved owes no arms, and a record's figure is re-run before it is a premise

Session 1513. Status: accepted. Prose: `doc/reviews/1513-review-1456-re-taken-under-the-performance-target.md`
(the measurements). Scripts: `doc/reviews/1513/` beside `doc/reviews/1456/`. Lines: `doc/todo/02`
section 0 item 3, and `doc/todo/_brief-template.md` *What the orchestrator does not write*. Keeps
ADR 1748 whole.

## 1. The question

ADR 1748 section 4 asked for review 1456's figures to be re-taken ten batches on. Over batches
seventy-two to eighty they say three things. The last four of those batches are the owner's
performance target.

- **The lock's queue is the round's largest term again.** It is 46% of round wall over the nine
  batches, 26% before the target and 59% under it. Rounds took 9 clock runs over seventy-two to
  seventy-six and 72 over seventy-seven to eighty. In seventy-eight to eighty, three-quarters of the
  queued wall passed while another round's clock run had asked or was holding. While a clock run
  waits, `tools/bounded.sh` grants no walk either lane, so a clock run waiting for lane 1 behind a
  large walk left lane 2 free for 1.0–1.5 h a batch while small walks queued for it. The budget now
  mostly measures that queue: in the ten rounds that used more than 75% of four hours, it was
  51–92% of the round.
- **Six-arm runs that could not move a digest.** Rounds ran 15. Twelve moved nothing. The one
  finding, a gpu page moved by an offscreen cull, came from a change to what is drawn. One round
  showed the evidence that makes a run unnecessary, and a later one ran its arms over a cut whose
  list digest was unmoved.
- **Premises got better, and what misses is stale figures.** Premise misses fell to 9 of 34 over
  batches 75–80. What still misses is figures and places, 17 of 23, and five of the eleven figures
  were quoted from an earlier record after the tree had moved.

## 2. Decisions

### 2.1 A round's clock figures are one hold

Section 0 item 3 gains one line. A round counts instructions while it explores (habit 87: a count
ranks and the clock decides). Then it takes every arm and every figure its verdict needs in one
`--clock` hold, interleaved, and the launch gate rides in it when the brief owes it. This costs a
round that finds a second question on the clock a second hold, which it then says. It gains back the
lanes for five siblings, which are stopped for every clock run's wait and not only for its hold.
Trap 149 says the hold keeps nothing else off the machine anyway, so a figure taken during a
sibling's build is "not judged" whichever hold it was in.

### 2.2 A change whose display list is unmoved on the corpus owes no six arms

The template gains one line against *Gates owed*. The arms compare pixels. A change that leaves
every first page's display list byte-identical cannot move them, because both backends draw from
that list. A digest of each list before and after (`callgrind_interpret --digest`) is the evidence,
or `display_list_census`'s counts for a change that can only move a bound. A change to what is
drawn still owes its arms, and the period's one finding was exactly that kind.

### 2.3 A record's figure is re-run before it is a premise

The template gains one line in *What the orchestrator does not write*. A record's figure belongs to
its own HEAD. The orchestrator re-runs its command at the brief's HEAD and quotes that, or puts the
claim in the hypothesis. This is ADR 1748 section 2.1 applied to the one source it did not name.

## 3. Declined

- **A cap on clock runs per batch, or a clock window in the wrapper.** Either one changes
  `tools/bounded.sh`, which belongs to an instruments slot, and the gate's freezing of lane 2 is a
  measured question for that slot (`doc/reviews/1513/lanes.py` prints it). The line in 2.1 asks
  rounds for what they control.
- **A shared arms-by-digest service.** It is the brief's hypothesis. 15 round runs in nine batches,
  12 of them clean, are paid more cheaply by 2.2 than by a service. Revisit if the count stays high
  after 2.2.
- **Records shorter than forty lines.** 40 of 53 sit at exactly 40. As ADR 1748 section 3 said, no
  round reads a record. This is recorded as a cost again.

## 4. Owed, and to whom

- `tools/state.sh batches` prints "no gate wall-time line" for every batch from seventy-four on. The
  bodies write `33 of 33 gates green` where the log prints `33 of 33 green`. Either the section
  admits the word, which belongs to whoever holds `tools/state.sh`, or the merge pastes the log's
  line as `doc/todo/02` section 8 item 4 says. Until one of them happens, `doc/reviews/1456/slots.py`
  is the reading.
- The briefs have left the template's caps: three of the last four common parts are over 60 lines,
  five of the last ten have a slot part over 25, and batch eighty-one's brief has seven "what changed"
  pointers where the cap is five. The owner's display loop is written out in the brief, where
  `doc/environment.md` already holds it. This is owed to the orchestrator's next brief, and the
  template already says to point.
- The next re-take is due at batch ninety or ninety-one. It runs the commands in the review's
  section 5.
