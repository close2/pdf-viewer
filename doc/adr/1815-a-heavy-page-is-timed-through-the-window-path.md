# 1815 — A heavy page is timed through the window path, in all four windows

Asked by the project owner on 2026-10-10, as the fourth part of one request: "I don't like that we
limit the number of operations, if we have valid files, which are not shown. As long as it isn't a
resource problem (only a time problem) we should allow it, and possible display a loading bar.
Obviously, if we find a way to improve loading, we should improve this as well. Please implement
this." Status: accepted. Session 1489. Context: ADRs 0607, 0657, 1605, 1808, 1809. Code:
`tools/drive-windows.sh` (`heavy_open`, step 80), `tools/state.sh` (`section_heavy`). Cites no
clause: an instrument, and the ledger is untouched.

## 1. What nothing measured

The owner's `quorra` run on `/tmp/demo_heavy_engineering_A1.pdf` took 16.23 s of user time to the
old cut, and no instrument of this tree had timed that path. `launch_path` times small documents;
`frame_budget` and `turn_path` time a frame's stages off the window; the corpus gates time a page in
a test process. What a reader waits for — a window, then a page in it, and whether the window hears
a key meanwhile — was taken by nobody, in any of the four windows, and the memory a window and its
worker reach on such a page was taken by nobody either. A count is not a cost, and neither is a test
process's clock the window's.

## 2. The decision

**`tools/drive-windows.sh`'s step 80, `heavy_open`, outside the default list, and `tools/state.sh
heavy`, which runs it as a clock walk** at nice 0 (trap 122) and then prices the page's crossing
with `examples/list_over_the_wire` as a small walk, a byte count. Per window it prints the launch to
the window mapped; the window's own first-frame stamp, the sentence `first_frame` already waits for,
which each of the four says only once a frame holding the page is presented; each process's
`VmHWM` and its sampled peak `Anonymous` (trap 35: the first counts libraries and the page cache, the
second is the program's own); what the page crossed as, from the confined window's own `frame:`
line; who drew it, in the window's words; and the longest a key waited.

**The key is ArrowRight, pressed every half second from the window mapping to the page landing,
and answered by the trace line that hands its `GoTo(Next)` to the viewer.** On a one-page document
that command changes nothing, so the instrument does not perturb what it times; a `HEAVY_FILE` of
more pages is refused by name rather than turned while it is timed. The answer is the window's own
line rather than a photograph, because what is asked is whether the window's loop ran, and a loop
that runs prints it.

**The windows run with the rayon pool they choose.** The lock's four threads are the walk's, and the
environment would hand them to the window, which is not how a person runs it.

**Before and after are two binaries, not two trees.** `HEAVY_BIN` names a directory of binaries
built elsewhere — an export of another commit, with its own target directory (trap 129) — and the
section then builds nothing and says it did not run the byte count, which is always this tree's.

**Figures go in the record and nowhere else** (ADR 0281). This ADR states what the instrument is
and what the first two readings found; the numbers are
`doc/history/1489-a-heavy-page-is-timed-through-the-window-path.md`'s.

## 3. What the first readings found, which the brief's hypothesis did not have

- **The page crosses as pixels**, as the brief predicted: its list is many times its scale-1 raster,
  and a list is the same size at every scale, so ADR 0607's rule sends the default window's page as
  `Payload::Raster`: the confined window's device is handed pixels to show, never marks to draw.
- **`quorra`'s own device refuses the page by its scene-byte budget** and draws it on the processor
  — on Xvfb's software device and on the owner's Radeon alike. The GPU path is not used for this page
  in the flagship either, for a reason that is neither the confinement nor the payload.
- **The window is deaf while the document opens.** `quorra`, `quorra-gtk` and the confined host
  answer no key until their launch trace says the document has joined, and `quorra-qt` maps no window until then. The confined host answers none until the
  page has landed. Slot 2's bar (ADR 1811) is drawn by a loop that, on this page at this commit, does
  not run for the first seconds; the instrument is what says whether it now does.

## 4. What it cannot see

The clocks are Xvfb's and a software device's, at nice 0 with siblings' builds on the machine, so a
figure is a reading with the load beside it and not a band; no gate fails on it. The owner's display
is a second arm only by hand (that record), and there no key is pressed, because a synthetic
key on a desktop in use can reach another window. `Anonymous` is sampled every tenth of a second and
can miss a shorter spike. The longest wait understates the deaf stretch by up to the half-second cadence, and by whatever
passed before the window mapped.
