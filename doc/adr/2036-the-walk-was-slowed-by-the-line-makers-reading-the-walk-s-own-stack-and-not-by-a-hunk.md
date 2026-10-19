# 2036 — The walk was slowed by the line makers reading the walk's own stack, and not by a hunk

Session 1600, slot 2 of batch ninety-five, `doc/todo/75` item 2's debt from ADR 2022 §4. Status:
**accepted as a finding**; the fix is measured and **not in the tree**, because the build it was for
is not (section 4). Context: ADRs 1878, 1884, 2022. Cites no clause and moves no ledger row. The
patches and the two counters are outside the repository, in `/home/AI/r1600-walk-false-sharing/`.

## 1. The instrument: the processor's own counters

There is no `perf` here, and callgrind models one core, so it cannot see what ADR 2022 could not
find. `perf_event_open` is open to user space (`perf_event_paranoid` 2), so two small programs read
the processor's counters for a command's main thread, which is the walk's: `pmc` counts events,
`psample` samples instruction pointers every 20 000 user cycles and the symbols attribute them. A
sample count does not move with the machine's load, as a duration does, so a layout can be judged
under a neighbour's build.

## 2. What it found: one binary, fast or slow by where its stack stands

The brief's bisection presumed a hunk. **No code needs to change for the walk to change speed**:
under `setarch -R`, pinned to the four full-clock cores, the change's own binary drew the witness's
fit with `encode: recording` near 86–110 ms or near 140–220 ms as an unread environment variable's
length moved the main thread's stack: slow at about three positions in four, with a period of 64 bytes. In the
slow positions the walk (`Device::encode_once`) took 10 800–17 300 samples against HEAD's 3 800–6 100,
and 3 400–12 600 of them sat on one of two instructions — a load from the walk's frame just after a
store to it (the `admit` rectangles handed to `place_line`, and the end of the made block's slice),
about forty cycles a line on 4 398 932 lines. With randomisation on, as a person runs it, the change
was slow in 10 of 12 processes and HEAD in none of 20 and none of 16 fixed positions.

**The cause is false sharing on the walk's stack.** The three line makers (ADR 1884) read, for every
line they make, values that live in the walk's frame: the `line_at` closure's captured slice header
(`&commands`, a local of `encode_once`), and the view behind `self.viewport`. A maker's read takes the
cache line shared; the walk's next write to a slot on that line has to take it back from another core,
and the load beside it waits. Which of the walk's per-line slots shares a line with those values is
decided by the frame's layout and by the stack's position modulo 64 — so it was ADR 2022's "same
instructions, same simulated misses", "slow in most processes and HEAD's in some", and slow every time
with randomisation off. A hunk only moved the frame; any hunk could have.

## 3. The fix, measured

`lines_ahead.rs` copies what the makers read into one value of its own, `Ahead` — the view itself,
the positions and the `line_at` closure — aligned to 128 bytes, so that nothing the walk writes can
share its lines (the adjacent-line prefetcher pairs 64-byte lines), and `encode.rs`'s two `line_at`
closures take their slices by `move`, so the slice header is inside it rather than in the frame. The
change with it: 32 of 32 stack positions and 12 of 12 randomised processes at HEAD's 2 800–4 500
samples, no instruction above 470, `encode: recording` 95–108 ms at 31 of the 32 positions (123 ms at one
whose samples, 4 467, say it was the machine and not the layout). The view's copy alone left 4 of 16
positions slow; the captured slice was the second reader. HEAD with the fix is HEAD (20 processes,
3 600–4 700 samples; one at 6 270 whose hot loop is the count of line commands before the walk, reading
the scene from memory, not the frame).

## 4. Why the fix is not in the tree, and what item 2 owes now

One `--clock` hold (load 1.9–2.8, the Radeon 890M through RADV, headless), HEAD's export against
the build with the fix (`line-lane-with-fix.patch`), interleaved, minimum of three rounds, two runs:

| | HEAD | the build with the fix |
|---|---|---|
| the fit's 1.25× step, `Cpu` | 301.4, 288.1 ms | 256.6, 256.9 ms |
| the fit's 1.25× step, `Compute` | 284.2, 285.5 ms | 251.7, 248.4 ms |
| `encode: recording`, the fit (two frames, two runs) | 85.7–91.0 ms | 83.7–98.0 ms |
| the window's first frame, `Compute` | 363.8, 367.0 ms | 411.7, 417.5 ms |
| the window's 1.25× and 1.5× steps, `Compute` | 124.0–127.3 ms | 123.8–131.6 ms |

**So most of ADR 2022's window regression was this cause too**: the step there went from 249–252 ms to
HEAD's own, with every line now an instance the vertex stage culls and the walk no slower for it. But
the brief's bar is a step faster than HEAD's *in the window*, and it is not: the window's steps are
equal, since HEAD's cull already sends little there (38 MB a step, the build 18, a transfer of 1–3 ms
either way), and its first frame is 48 ms slower,
since it sends the page's 264 MB of lines where HEAD sends what the view admits. The fit's step is
31–45 ms faster, which is the zoom the window does not draw. So the build stays out of the tree and
`doc/todo/75` item 2 stays owed — no longer for an unexplained walk, but for what it buys: nothing in
the window until item 3 builds the op list once, when the first frame's send is paid once a page
rather than a step's saving forgone. **The fix returns with the build and not before**: HEAD's frame
does not strike the hazard (0 of 20 processes, 0 of 16 positions), and an optimisation no measurement
of the shipped tree justifies is not made (principle 2 against 4). What any later change to
`encode_once`'s frame inherits is the instrument: a walk that slows with equal instructions is first
sampled per instruction, a few positions of its stack apart, before anything is bisected.
