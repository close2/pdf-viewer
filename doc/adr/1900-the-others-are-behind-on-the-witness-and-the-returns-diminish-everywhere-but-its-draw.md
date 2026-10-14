# 1900 — The others are behind on the witness, and the returns diminish everywhere but its draw

Session 1532, the step back of batch eighty-four on the project owner's target of 2026-10-14:
"[c]ontinue improving performance (and fixing bugs) until you reach diminishing returns. Then switch
to code improvements/refactoring". The owner's first complaint was "[w]e are currently slower in
displaying the initial frame than pdfcraft and okular". Status: **accepted**. It measures HEAD
`77fa741b` against four other programs, gives the verdict the owner asked for, and builds nothing.
Context: ADRs 1845, 1873, 1888; `doc/todo/69` §7. Witness: `demo_heavy_engineering_A1.pdf` from
<https://github.com/cyfung1031/BergPDF> at `a9f4bac2` (`sample/`), linked from
<https://github.com/storytold/pdfcraft/issues/563>: one A1 page, 39 533 504 bytes, MD5 `166bb8be…`,
not in the repository. It cites no clause and moves no ledger row.

## 1. What was measured, and how

- **The programs.** HEAD's `quorra`, `quorra-gtk`, `quorra-qt` and `quorra-transform`, built
  `--release` from an export. PdfCraft upstream `e4028b3c` (2026-10-13), built `--release` (`pdfcraft`,
  `pdfcraft-cli`). The system's okular, `mutool` and `pdftoppm`.
- **The documents.** The witness, and the launch gate's six. For the turn into the witness,
  `tmp/r1518/turn.pdf`: a light page, then the witness's page.
- **Headless** means one process from spawn to exit, rendering to PPM or PAM at the window's fit
  (1 000 px high; 42.75 dpi for the witness). Peak memory is the child's `wait4` rusage.
  `RAYON_NUM_THREADS` is unset, so each program takes its own threads.
- **Windows** were all timed by one instrument, not by each program's own clock. It photographs
  the window with `xwd -id` every 9–32 ms (median per run) and keeps every picture that differs
  from the one before. A phase ends after a quiet window with no change. Its figure is the first
  picture equal to the last one, counted from the spawn or from the key. `quorra`'s own trace
  agrees within one picture: first present 0.808 s against 0.843 s.
- **Keys.** Each program's own key for the next page and for zoom in (`Right`; `plus`, `ctrl+plus`,
  `ctrl+equal`).
- **Window sizes.** Each program opens at its own size. On the owner's display: `quorra`
  1275 × 1594, gtk 1046 × 1146, qt 1728 × 1116, okular 1594 × 1718, pdfcraft 2295 × 1466.
- **Sittings.** One `--clock` hold (1 399 s): headless three rounds, Xvfb two runs, the owner's
  display one run. Load was 1.3–3.5, and every run was unpinned at `nice 19`. A second hold
  (244 s) re-took the witness on the owner's display with a 15 s quiet window (§4).

## 2. The figures

**Headless, the witness** (ms, min–max of three; CPU and peak memory beside):

| program | fit | 2× fit | CPU at fit | peak |
|---|---|---|---|---|
| `quorra-transform` | **1 130–1 156** | **1 280–1 378** | 11.5 s | **1 856 MiB** |
| `mutool draw` | 4 705–4 757 | 4 743–4 764 | 4.7 s | 278 MiB |
| `pdfcraft-cli` | 5 714–5 748 | 7 011–7 091 | 5.7 s | 425 MiB |
| `pdftoppm` | 10 053–10 216 | 10 314–10 423 | 10.0 s | 22 MiB |

**Windows, the witness** (s from spawn or key; owner's display, then Xvfb):

| | `quorra` | `quorra-gtk` | `quorra-qt` | okular | pdfcraft |
|---|---|---|---|---|---|
| first frame, owner | **2.04, 2.06, 2.26** | 1.72 | 1.70, 1.75, 1.76 | 11.34 | 6.95, 8.04 |
| first look, owner | 0.63–0.86 | | | 0.68 (drawn as read) | 0.23 ("Rendering…") |
| first frame, Xvfb | 1.86, 1.87 | 1.71, 1.84 | 1.58, 1.64 | 11.36, 12.25 | 7.23, 7.25 |
| zoom step, owner | 0.74 | 0.46 | 0.41 | 11.53 | not measured (§4) |
| zoom step, Xvfb | 0.91, 0.96 | 0.42, 0.42 | 0.39, 0.40 | 11.87, 14.87 | 2.81, 2.96 |
| turn into it, owner | 1.95 | 1.61 | 1.83 | 4.44 | 1.99 |
| turn into it, Xvfb | 1.80, 1.87 | 1.68, 1.73 | 1.58, 1.67 | 1.82, 3.24 | 0.03, 0.04 (drawn ahead) |

**The six ordinary documents**, range over the six (s; owner's display, one run each):

| | `quorra` | `quorra-gtk` | `quorra-qt` | okular | pdfcraft |
|---|---|---|---|---|---|
| first frame | **0.071–0.184** | 0.223–0.251 | 0.117–0.178 | 0.231–0.444 | 0.264–1.402 |
| turn (three documents) | 0.049–0.078 | 0.036–0.039 | 0.057–0.066 | **0.027–0.028** | 0.036–0.070 |
| zoom step | **0.027–0.064** | 0.038–0.076 | 0.029–0.063 | 0.035–0.416 | 0.070–0.208 |

Headless, at the fit, the ordinary documents take `quorra-transform` 11–42 ms, `mutool` 13–51,
`pdftoppm` 16–126 and `pdfcraft-cli` 7–77. `pdfcraft-cli` is the fastest on four of the six:
`WTPDF` 10–14 ms against 20–22, `xfa_filled_imm1344e.pdf` 7–11 against 14, `AN001` 11–12
against 12–13, and `opt_demo.pdf` 11–12 against 11–16. `quorra-transform` wins `ISO 32000-2`
(22–26 ms against 36–66) and `bug1815476.pdf` (35–42 against 71–77).

## 3. The hypothesis, and where each comparator is ahead

The brief's hypothesis was: HEAD's `quorra` is ahead of okular and pdftoppm on the witness's first
frame, and behind on nothing else measurable. **The first half holds**: 5.0–5.6× against okular
in the window, and 8.7–9.0× against `pdftoppm` headless. **The second half does not.** Four figures
are behind or even:

1. **pdfcraft-cli, headless, on small pages: 2–10 ms, in the read.** At 1 dpi there is next to
   no draw. On `WTPDF` that leaves 11.9–13.6 ms on one thread against 7.0–7.5. Callgrind counts
   97.2 M instructions against 36.3 M, and 60.2 M of them are `pdf_model::image::convert_three`:
   page 1's 350 × 350 `ICCBased` JPEG is converted sample by sample through
   `icc::Profile::to_xyz_with`, about 490 instructions a sample. PdfCraft builds one 8-bit
   transform (`moxcms::create_transform_8bit`, 3.5 M) and applies it. Process start on the pool
   adds about 2 ms (`AN001` at 1 dpi: 5.6–7.1 ms on the pool, 3.6–3.8 ms on one thread).
   **Nobody sees this in a window**: the device's bring-up is longer than the read on every row
   (ADR 1544).
2. **okular's turn: 0.027–0.028 s against `quorra`'s 0.049–0.078 s.** The instrument's own floor
   (an `xdotool` key and one picture) is 20–30 ms. `quorra`'s trace puts the real frame of the
   `ISO 32000-2` turn 16 ms after the key, after a stand-in at 1 ms. The last change is a second
   present at 44 ms. **The difference is one extra present, inside a refresh or two**, and no
   figure a person can see.
3. **okular's first look at the witness: 0.68 s**, drawn as poppler reads. `quorra`'s first look
   is 0.63–0.86 s (`page shown as it is read`, 4 101 commands, at 0.66–0.75 s in its trace).
   **Even within a picture, to 0.2 s behind.**
4. **Memory and CPU on the witness.** `quorra-transform` peaks at 1.84–1.86 GiB and spends
   11.5 CPU-seconds. `mutool` peaks at 278 MiB in 4.7, pdfcraft at 425 MiB in 5.7. That is the
   price of ADR 1873's divided read (48 pieces' lists in flight) and of the pool. No gate holds a
   heavy page's peak.

pdfcraft's "first frame" on the witness is not faster. Its window shows "Rendering…" over a blank
page at 0.23 s and the page at 6.95–8.04 s. In round 1497 this tree's first frame read 6.73 and
5.20 s on the owner's display (ADR 1845), the same order as pdfcraft's. **At HEAD the owner's
complaint no longer holds** against either program: 2.04–2.26 s. The toolkit windows are behind nothing, so slot 1 is not the cause of anything here;
on the witness they are the quickest of the three windows.

## 4. What the instrument got wrong once

The first sitting gave pdfcraft's witness first frame as **0.561 s** and its zoom as **0.083 s**.
The settled picture was the "Rendering…" placeholder: pdfcraft draws nothing for longer than the
5 s quiet window. With a 15 s window, the first frame read 6.95–8.04 s. The zoom key reached no
window in that second sitting (KWin activation of a window idle for 15 s; all three programs, both
runs), so pdfcraft's zoom on the owner's display stays unmeasured. Its Xvfb figure, 2.81–2.96 s,
is the page (looked at). **A quiet window shorter than a program's longest silent stage measures
its placeholder.** Every settled picture of the witness was opened and looked at before it was
used.

## 5. The verdict: diminishing returns, except one build

- **Ordinary documents: reached.** `quorra`'s first frame (0.07–0.18 s) is the quickest of five
  programs on five of the six documents; on `bug1815476.pdf`, `quorra-qt` is 6 ms quicker. Its turns and zoom steps are within one or two refreshes of the instrument's floor.
  The read pdfcraft wins by 2–10 ms is hidden behind the device's bring-up. A lookup-table
  conversion for `ICCBased` image samples would take about 50 M instructions off `WTPDF`'s page,
  and no window figure would move. It is priced here and not picked.
- **The witness's zoom: reached.** It is 0.41–0.74 s against okular's 11.5 s and pdfcraft's
  2.8–3.0 s, and slot 4's kept line instances are the next small step.
- **The witness's first frame: not reached, by one build.** `quorra`'s trace on the owner's
  display has the page interpreted at 0.77–0.95 s and first present at 2.01–2.23 s: "page drawn
  while it was read, finished on the processor in" 1 235–1 272 ms. On Xvfb it is 0.70–0.72 s and
  1 076–1 109 ms. That is `Underway`, the drawer on one thread. The
  pooled draw is 490–525 ms (ADR 1888), and `quorra-transform` does read and draw together in
  1.13–1.16 s. The window is therefore about 0.9–1.1 s slower than the tree's own headless floor,
  and slot 3 of this batch is the build that closes it: **2.04–2.26 s → about 1.0–1.5 s**,
  projected (a read of 0.77–0.95 s, then at most the pool's 0.49–0.52 s), not measured. The read is then the floor: 0.27–0.31 s pipelined (ADR 1888), bounded
  by an inflate that does not divide.
- **After slot 3 lands, switch.** The batch after it measures the witness's first frame once more.
  If that frame is within 0.2 s of the headless `quorra-transform` figure, the next batch is
  code improvement. The two exceptions it keeps are the memory figure (§3.4), which a gate should
  hold first, and the bug in §6.

**The refactoring targets a code-improvement batch would take**, each counted by a command:

- The two toolkit hosts, `crates/viewer-gtk/src/host.rs` (7 865 lines) and
  `crates/viewer-qt/src/host.rs` (6 401). Each carries its own copy of the turn, the drawer and the
  open sequence beside `viewer-host`. Count with `wc -l`, and `grep -c 'Ahead\|Underway'` on each.
- `quorra`'s `renderer.rs` (3 569 lines) and `stale.rs` (3 686): the lane choice, the drawer and the
  stand-in frame have grown over twenty ADRs. `wc -l crates/viewer-ui/src/bin/quorra/*.rs`.
- The comments that carry their history, which `CLAUDE.md` counts with
  `grep -rE "hundred-and-|session" --include=*.rs crates tools`: 245 lines today, 13 files in
  `viewer-ui`, 12 in `pdf-model`.
- The largest single-file modules on a hot path: `render-cpu/src/lib.rs` (7 513 lines) and
  `pdf-model/src/image.rs` (8 949).

## 6. A bug the measurement found

**On Xvfb, `quorra-gtk` opening `bug1815476.pdf` maps no window in 5 of 8 starts (and in one of the
two hold runs), and never recovers.** `WTPDF` and `AN001` map in 6 of 6. In a hung process,
27 threads are in `futex` waits and the main thread polls. GTK has spawned `dbus-launch` (left as a
zombie) because Xvfb has no session bus. `pdf-sandbox-worker`, spawned for the page's CCITT image,
holds both ends of two pipes the main process also holds (fds 6–9), and it waits on its own
request pipe. This reads as GLib's `dbus-launch` capture pipes reaching the worker, which then
never gives them their end of file. The mechanism has not been confirmed in the code. On the
owner's display there is a session bus, and the window opened. It is an Xvfb start, but it is a
deadlock: the next host round reproduces it (eight starts of `quorra-gtk bug1815476.pdf` on a bare
Xvfb, each given 10 s to map) and reads `pdf-sandbox`'s spawn (`lib.rs:549`) against GLib's.

## 7. Not measured here

- **pdfcraft's zoom on the owner's display**, for §4's reason.
- **mutool and pdftoppm in a window**: neither has a window here (`mupdf-gl` is not installed),
  so they are headless only.
- **A pinned run.** Every figure is unpinned, so it is a draw from the fast-core/slow-core
  lottery (`doc/performance.md`, the launch path). The rankings above hold by more than that
  lottery's 45–50 %, except where §2 calls two programs even and in §3.2.
