# Round 2149 — audio is opened, and a sound is decoded in a confined worker

The owner's A946 (RFC 0015 section 12, all six recommendations): audio opened alone, video kept
closed behind it, U3D's trigger restated to ECMA-363's third edition, a PRC trigger written. This
round wrote the scope text and built RFC 0015 section 5's first step.

## What it did

- `CLAUDE.md`'s clause 13 bullet and `doc/scope-ratifications.md`'s entry: retitled "the playing of
  video and the rendering of 3D artwork"; audio decoded and presented by this program in scope;
  three triggers (video behind audio, U3D third edition, PRC with ISO 14739-1 held); rich media,
  Go-To-3D-View, Rich-Media-Execute and §13.5 placed as A946 item 6 says.
- `pdf_sandbox::lockdown::Profile::Media`: `Profile::Script`'s list, a 64 MiB ceiling (peak measured
  20.3 MiB). `pdf_sandbox::Allowance`/`Ended` and `Canceller::process_id` made public.
- `crates/pdf-media-worker`: `MediaWorker` (started at the first sound, 1 s processor budget per
  request, wall ceiling 10 s), `serve`, `wire` (runs of whole frames ≤ 256 KiB, reply counts checked),
  `sound` (Raw, Signed 1–32 bits, G.711 µ-law and A-law into top-aligned `i32`).
- Fuzz target `media_sound`, `fuzz/seed_media_sound.py`, its `seeds.sh` arm and `doc/verify.md` line.
- ADR 3297: the decision, the measurements, and principle 3's justification for the sink's C.

## Ledger

No status moves. §13.3 was already `implemented` (the hand-off); its note, code and tests now name
the worker. §13.1 and §13.2.1 stay `out-of-scope` (they describe video's engine too), their notes
say audio is in scope; every out-of-scope note names the exclusion by its new title.

## Gates

**Gates.** fmt exit 0. clippy `-D warnings --all-targets` over `-p pdf-media-worker -p pdf-sandbox
-p confined-transport -p conformance` exit 0 on the second run; the first found an unfulfilled
`expect` in the new tests. `pdf-sandbox --bins` exit 0. nextest `--lib --bins --tests` of those
four (`xvfb-run -a`): 582 run, 581 passed, 1 failed, this record's missing paragraph; the first
run also failed the install list and two G.711 table designations, both fixed. rustdoc exit 0
for all four. `fuzz` `cargo check --bins` exit 0; `media_sound` ran 2,015,736 inputs in 61 s,
no crash. `cargo test -p conformance --no-fail-fast` exit 0, run after this paragraph was written.

## Habit

Read the row before moving it: the brief's "§13.3's rows from out-of-scope" was a premise the
ledger did not hold — §13.3 was `implemented` on the hand-off already.
