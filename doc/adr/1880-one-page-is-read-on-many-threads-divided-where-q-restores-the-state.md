# 1880 — One page is read on many threads, divided where `Q` restores the state

Session 1522, the build ADR 1873 §5 chose, on the project owner's target of 2026-10-10. Status:
**accepted** and **built** in `pdf-model`. Context: ADRs 0362, 1507, 1855, 1873, 1888. Code:
`crates/pdf-model/src/content/divide.rs` (new: `Plan`, `read_page`, `Interpreter::{read_divided,
join}`, `Division`, `last_reading`), `content/run.rs` (`run_from`, `Composed`, `Leftover`,
`end_of_stream`), `content/reader.rs` (`ContentReader::{piece, read_to_end_within}`),
`content/list_budget.rs` (`ListBudget::{for_piece, absorb}`), `content/transparency.rs`
(`Readings::{carried_by_join, absorb_piece}`), `content/report.rs` (`Notes::absorb`),
`progress.rs` (`Parts::spoil`), `pdf_render::DisplayList::append`. Tests:
`tests/drawn_while_read.rs` (three fixtures and a corpus sweep). Instrument:
`examples/read_split_price.rs`'s `built` line. Witness: `/tmp/demo_heavy_engineering_A1.pdf`.
Clause: §8.4.2; no ledger row moves (the partial-row word yields to the owner's target).

## 1. The premise and the hypothesis

**The premise held**: `crates/pdf-model/src/content` and `crates/pdf-syntax/src` were unchanged
since `c6fb18fb`, and HEAD read the witness in 1 768–2 394 ms headless in this round's runs (load
from five siblings), round 1518's 1 739–1 929 ms on a quieter machine.

**The hypothesis did not hold in its letter.** ADR 1855's parts are copies of their commands'
headers, and an `Interpretation` is one `DisplayList`: handing a piece to the drawer as a part does
not spare the join, it adds a second copy beside it. The join is built and priced instead (§3): one
reservation, and each piece copied in as it arrives while the later pieces are still being read.

## 2. Decision

1. **The state at a cut is composed, never assumed.** The plan lexes the inflated stream in
   chunks at once; each chunk records its effect on the stack — how many levels it pops below its
   start, and per level it ends at the statements whose effect lasts: every `cm`, `ri` and clip's
   path object in order, the last of each line parameter, colour space and colour, and §9.3.1's
   text state parameters. The effects compose in order. A later piece's state is the composed
   spine's own statements run by a scratch interpreter — the producer's bytes in the producer's
   order, a `q` between levels — whose state stack and clip table the piece begins with.
2. **A cut is after a `Q`, where only the state is open**: no path or pending clip (§8.5.2.1), no
   text object (§9.4.1) — every text object is the last piece's, so no cut follows the first `BT`
   — no marked-content sequence, compatibility section or inline image (those decline, item 4). A
   chunk begins just after a path-painting operator, which ends a path object (§8.5.3.1), so a
   clip's path object is never split; the chunk before must end on that same token, or the plan
   declines.
3. **What is the page's stays one.** Each piece may spend `MAX_LIST_BYTES / pieces` and its tail
   is settled, so the joined charge is the page's (the witness: 776 097 064 bytes both ways); the
   notes are a set and absorb as one; the counts add; §11.6.4.3's record joins where one reading
   was in force throughout (`Readings::carried_by_join`); the clips join through `add_clip`, so a
   region already in the table keeps its identifier, and a piece's commands are renumbered; ADR
   1855's hand-over leaves from the first piece on the calling thread, and a division given up after
   a part was handed spoils it. The font cache across interpretations is shared.
4. **Exact or not at all.** Anything the join does not carry declines, by a sentence
   (`last_reading`), and the page is read whole: on one thread from the whole bytes where the plan
   lexed every token, through a new window where it met something a window reads differently (a
   token past `CEILING`, an inline dictionary, an inline image). Declining today: an `/ExtGState`, marked content,
   an inline image, a compatibility section, text before the last piece (inside a form), a soft
   mask, a transfer function, a press named, an unmatched `Q`.
5. **Where**: a stream of at least `DIVIDE_FROM` (1 MiB) encoded, compositing on the device, no
   ledger; four pieces a pool thread (`PIECES_PER_THREAD`), each at least a mebibyte. Reading whole
   is what a division needs and what ADR 0362's window refused, so the whole read is bounded by
   `INFLATION_BOUND` (128) times the encoded length; past it the page is read through its window.

## 3. Measured

**The corpus** (`every_tracked_first_page_read_in_pieces_is_the_page_read_whole`, four pieces of
every stream however short): 974 documents, 50 first pages divided, every one stating what the whole
reading states — the list by its equality, everything else by its rendering, the charge included —
and 49 of them drawn while read byte for byte the page drawn whole; every declined page the same
page too. The list's rendering also prints each path's memo of its own hull, which differs on one
divided page (`bug1721218_reduced.pdf`) because a later piece asked its own copy of a clip; equality
ignores the memo by design, and the witness's digest holds.

**The witness**, headless, `read_split_price`'s `built` line (the page read as `interpret` reads it
against `Division::NEVER`, interleaved in one process, three rounds), on the second lane beside five
siblings' walks — **exploration figures, not a clock hold** (§4): on twelve threads, 48 pieces,
**585–697 ms against 1 833–1 902 ms whole, 2.6–3.3×**; on four threads, before the pieces were four
a thread and the join overlapped, 1 514 ms against 1 883–2 834 ms. Its stages, timed once in a
build that printed them (twelve threads): the inflate 145–158 ms, the plan 98–117 ms, the pieces
about 300 ms, the join 150–164 ms before it ran beside the pieces. `callgrind_interpret --digest`
holds `07eb7445…68e3` and 776 097 064 bytes charged at four and at twelve threads; the joined list
equals the page's by `DisplayList`'s equality in every round.

## 4. What it costs, and what is left

- The inflated stream is resident during the read (278 MB on the witness) beside the pieces' lists,
  and a stream that would inflate past 128× its encoded bytes is read once to that bound before the
  window reads it again.
- The serial terms left are the inflate, the plan and the join's copy, in that order; ADR 1888
  (round 1526) prices the inflate pipelined into the pieces and chose it as the next build.
- **Not measured: the clock hold.** Its arms — the read at the window's 24 threads, `launch_path`,
  `quorra`'s first frame under Xvfb and on the owner's display, HEAD's binary (`d8f0cff8…`, round
  1525's build of `56e42ea5`) against this one's (`db072273…`) — are staged in round 1522's
  `scratchpad/r1522/hold.sh` and `tmp/r1522/owner.sh`; the hold queued for 3 h 5 min behind another
  round's corpus arms and was given up at the round's budget.
- The drawer sees only the first piece's parts before the read ends, and a read this short ends
  near ADR 1855's `GLIMPSE_AFTER`: the first frame is the whole page drawn after the read.
- Carrying the declines: an `/ExtGState` needs the transfer channel, the soft-mask table and the
  readings joined with offsets; marked content needs its spans offset as the readback's are not.
