# 1104 — The wheel travelled a hundred and forty-six lines and the zoom moved one step

2026-09-15. ADR 1118. One trace: `tmp/trace.txt.gz`, 4 844 lines, the owner's run of 21:50. Files:
`viewer-ui/src/bin/quorra/{sidebar.rs,app.rs}`, `viewer-ui/src/bin/quorra.rs`.

**The timeline.** *Release Snapshot cb61f751 · close2\_pdf-viewer · GitHub.pdf*, 2 pages, on an Intel Arc
(MTL) over Vulkan — **not this machine**, which `lspci` says is the Strix `doc/environment.md` records, so
every device figure here is Intel ANV's and is not reproducible on this one. Device up 54.6 ms (pipelines
14.5 ms, waited for by nothing), first present 350.3 ms, 245 frames and 239 presents to a close at 27.9 s. A
wheel scroll at 2.7–3.1 s, then **seven Ctrl-held gestures** at 3.6, 6.2, 8.3, 12.6, 15.5, 17.8 and 20.7 s,
then a pointer sweep. No warning, refusal, fallback or repack.

**The finding: 579 Ctrl + wheel events, 146.4 lines of travel, one `zoom` line in the file.** Per gesture,
events/sum|lines|/zooms: 158/28.1/0, 100/26.4/**1**, 26/9.2/0, 39/21.0/0, 179/29.7/0, 66/31.7/0, 11/0.3/0.
The trace prints every window event and a `zoom` only where a command is dispatched, so it said this the
moment it was written — and two of its own headline figures **are** that silence: `p99 3513.3 ms` and `max
6713.3 ms` between presents are 3.090→6.603 s and 12.765→19.478 s, spans in which the owner cranked the
wheel at a window with nothing new to show.

**The mechanism**, `App::wheel`'s Ctrl arm: `LineDelta(_, lines) => { self.pinch = 0.0; lines.trunc() }`.
`winit`'s X11 backend divides an XInput2 smooth-scroll valuator by the axis increment, so a high-resolution
wheel sends a *fraction* of a line per event — the owner's quantum was 0.026 981 818 of one, a
thirty-seventh, and `trunc()` of that is zero, once per event, with the accumulator reset on the way past so
nothing carried either. The step that did fire is the one event whose delta reached 1.0 at 6.602; what
followed it was blameless — five approximated frames over 76 ms, then the real one, as ADR 0443 says.

**Before / after**, on the trace's own device: four lines of travel arrive as 149 events of a thirty-seventh
each and bought **0** steps; they now buy **4**, and `zoom_steps`'s test reproduces the old arithmetic beside
the new so both figures are in the tree. `WHEEL_ZOOM_PIXELS` is unchanged at 50 px a step (125 px is 2), a
whole notch is still one step on its own event, a reversal cancels, and a non-finite delta no longer poisons
an accumulator a per-event truncation could not poison.

**Not widened, named instead.** `0 of 245 frames replayed a retained encode` is no defect: `EncodeKey` carries
the transform's bits, each of the 40 real frames had a different one, and `retained.rs` says so. Frame 1's 275
uploads, and encode p90 23.9 / max 55.5 ms — the 77.7 ms the approximation line predicts — are round 1099's.

**Gates.** `fmt -p viewer-ui` 0; `nextest -p viewer-ui` 133 passed 0; `-p conformance` 0; `nextest -p
render-gpu` 38 passed 0; `--release -p viewer-ui --test launch_path --ignored` 26 banded, 0 outside, 0.
`RUSTFLAGS="-D warnings" clippy -p viewer-ui --all-targets` is red on **siblings' files alone** —
`variable_text.rs` and `quorra/files.rs`; with `--no-deps` mine is clean.
