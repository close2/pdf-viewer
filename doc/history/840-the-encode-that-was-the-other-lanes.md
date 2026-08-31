# 840 — The encode that was the other lane's

The round was pointed at `doc/todo/47-the-encode-term.md`, whose revisit condition — take up
device-resident records when encode is the largest term of a zoom step — round 839 had reported
met: 132.5 ms per resize step on Entwurf, 129.0 of it quorra's `encode` (ADR 0766 §3). The first
thing re-derivation found is that the instrument and the window disagree about which lane draws
that gesture: `examples/zoom_frame` built its device on quorra's default `Coverage::Cpu`, and
`surface::lane_for` takes `Coverage::Compute` for any moved view on a real adapter — which an
arm-2 resize step is on every step. ADR 0767 is the reading.

## What was done

- §5 first: the release binaries were older than `HEAD` and were rebuilt and installed before a
  number was taken. This is also a fifth round, so §2 ran whole.
- `examples/zoom_frame` gained `ZOOM_FRAME_COVERAGE={cpu,gpu,compute}` (default `cpu`, so every
  earlier table stays comparable) and a `record-replayed` marker on the frame line beside the
  existing `replayed`. That is the round's one build; nothing shipped changed.
- One A/B sitting, the round's cap: both lanes on `tmp/Entwurf.pdf` page 1 on the real 890M,
  headless, ADR 0766's own resize-sized sequence, arms interleaved A B A B, plus one
  `ZOOM_FRAME_ENCODE_PHASES=1` run for the shares and the ISO page as the small witness. The
  Cpu arm reproduced 839's row to the tenth (132.5 total / 129.0 encode), so the correction is
  of the reading, not the run.
- The numbers, and the decision they carry (ADR 0767): on the lane the window takes, the step is
  **63–66 ms** — kernels 44–46 (count 13.8–17.5, emit+deposit 28.2–31.2), host encode 9.4–10.1
  and `record-replayed` on every warm step, residency+records 4.0–4.2, transfer ~5. Encode is
  not the largest term; the kernels are 2.3× everything the host does. **Device-resident records
  stay parked**: the shape removes at most ~19 ms of host terms and none of the kernels, and
  quorra ADR 0091's two design debts are still unargued. `doc/todo/46`'s flatten-from-quadratics
  remains the only item with the step's magnitude, and its success is what would make the revisit
  condition true for real.
- `doc/todo/47-the-encode-term.md` restated around the corrected numbers, with the condition now
  naming the lane it must be measured on; `doc/todo/47-the-resize-frames.md` §3 corrected;
  `doc/todo/46` gains the re-measured decomposition; `doc/todo/README.md`'s two lines follow.
  Nothing is owed to quorra — their ADRs' numbers reproduced, and record replay is observed
  earning ~8 ms per warm step against the cold walk on exactly the gesture ADR 0087 measured it
  "no win" on.

## Second track

`doc/conformance/ledger.toml` §9.8.3.1, `partial`, from the top of the blame-ordered reading
list. Its exculpation — "[f]or an embedded CIDFont, which is what every corpus CIDFont that
draws is, neither can change a glyph" — is refuted by its own sibling row's witnesses:
`noembed-eucjp.pdf` and `noembed-sjis.pdf` carry non-embedded `CIDFontType2` fonts (checked in
the files) drawn through `loading.rs`'s composite substitution route, the same route §9.8.3.2's
`/Panose` reading chooses a face for. So `/Lang` is exactly the evidence the note says it is
and is read by nothing in the face comparison; the row stays `partial` with `/Lang` now an owed
input to substitution rather than a moot entry. `/CIDSet`'s exculpation stands.

## Gates

The full §2 sequence (a fifth round), all green: fmt, clippy under `-D warnings`, nextest
(2837 passed, 18 skipped), doctests, the two `fuzz/` lines, the sandbox and hayro builds, and
every gates-profile corpus line — corpus, oracle, the three text gates, both censuses, dates,
xmp, jpeg2000, the quorra gate, `fixed_documents` (41 checked, 0 absent) and
`cargo test -p conformance`. The `quotations` and `pointers` sweeps ran for the moved
documents; every divergence either prints is a historical document quoting a misquotation on
purpose, none this round's. The quorra gate's second coverage lane was not run: the change in
`render-quorra` is an example knob that no shipped pixel path reads. The sequence ran after
the measurement sitting, nothing beside it.
