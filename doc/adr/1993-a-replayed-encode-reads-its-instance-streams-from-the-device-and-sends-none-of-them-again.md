# 1993 — A replayed encode reads its instance streams from the device and sends none of them again

Session 1578, the first step of ADR 1992's ordered list (`doc/todo/75` item 1), for the project
owner's ask of 2026-10-19 about a page kept on the device. Status: **accepted** and **built** in
`raster-gpu` (`device/staging.rs`'s `KeptInstances` and `instance_streams`, `retained.rs`'s
`Held::instances` and `RetainedScene::resident_instance_bytes`, `device/render.rs`'s `reported`).
Context: ADRs 0351, 1877 §5, 1992; raster's ADRs 0048, 0095. Tests: `raster-gpu`'s
`retained_replay.rs` (`a_replay_counts_what_the_encode_counted`,
`a_replayed_line_page_is_the_page_that_was_encoded_and_sends_no_line`). Cites no clause and moves no
ledger row.

## 1. What was found

A retained encode (raster's ADR 0048) kept its instance streams on the host and nowhere else, so
every replay — the frame of an unchanged page that ADR 0351 keeps the scene of — created three
vertex buffers and scheduled every instance byte again. ADR 1992's instrument names the stream:
on the witness a replay sent 277 402 546 bytes under `Cpu` coverage and 304 700 013 under `Compute`,
of which 264 499 408 were its instances, and the replay's `transfer` was 27.6–33.8 ms. Nothing a
pass does writes an instance buffer, so the bytes a replay sent were the bytes already there.

## 2. Decision

**The first frame that draws a retained encode leaves its three streams on the device beside it**,
and every replay of that encode binds them. The cell is `retained::Held`'s, so the streams live and
die with the encode they were made from: a new encode (another view, a new scene, a repack, another
device) is a new cell and sends its own once; `forget`, `set_scene` and a replaced encode give them
back. `Device::render`, which retains nothing, keeps nothing. A frame run again after the compute
lane's capacity grew (raster's ADR 0095) finds its streams already sent, so it reports its first
attempt's instance shares (`reported`) rather than the zero its re-run sent.

**The price is device memory between frames**: the encode's instance bytes, which its first frame
had allocated and the frame budget had admitted, now held while the encode is retained rather than
for one frame — 264 499 408 bytes on the witness, 189 752 on page 101 of the EC3 specification.
`RetainedScene::resident_instance_bytes` says it, beside `retained_bytes` for the host's copy, so
the caller that decides to hold an encode can count both. The peak is unchanged: a replaced
encode's streams are dropped inside `RetainedScene::prepare`, before the new frame allocates.

## 3. What it moved

The hold of ADR 1992 (load 6.3 → 2.5), HEAD's export against this one's, `zoom_frame` on the witness
at the fit, 1.25× and 1.25× again (the replay), minimum of three rounds, two runs a lane:

| the replay of the 1.25× frame | HEAD | this |
|---|---|---|
| `Cpu`: frame | 157.3, 156.9 ms | 140.6, 132.5 ms |
| `Cpu`: transfer, bytes | 27.6, 30.0 ms; 277 402 546 | 0.7, 0.7 ms; 12 903 138 (the sheet) |
| `Compute`: frame | 169.6, 164.3 ms | 127.2, 171.5 ms |
| `Compute`: transfer, bytes | 30.4, 33.8 ms; 304 700 013 | 1.1, 1.2 ms; 40 200 605 (the sheet, the records, device-only) |

A second hold (load 5.8 → 5.4) took two turn rows at scale 1 and 1.25, five rounds, two runs a lane:
22060's replay sends 2 272 464 bytes where HEAD's sent 2 276 188, bug1743245's 36 300 where 53 264 —
the difference each time its instance bytes — and both frames stay 0.8–5.8 ms, the steps 18.8–22.1 ms
against HEAD's 19.2–21.6. All figures are the Radeon 890M through RADV, headless.

The transfer is the measured saving, 27–33 ms a replay of the witness; the frame moves by it less the device's own
spread (`execute` 90–123 ms over the eight runs). The zoom step itself encodes and sends what it sent:
its streams are a new encode's. Under `Compute` the first frame's re-run no longer sends the streams
a second time, and its `transfer` reads 1.5–1.8 ms where HEAD's read 29.2–29.4.

## 4. Proof that no pixel moved

Every frame's digest in the hold is HEAD's: `077df9c0673f9ce0` and `45e141463571ded7` under `Cpu`,
`e59f8ea49e179fdb` and `a8bee10e7b0230b1` under `Compute`, at the fit, the step and the replay, in every
round of both runs — six a lane in each arm. `retained_replay.rs` holds a replayed text page, artwork page and page of
the line lane's marks (every cap, four directions, a hairline, a dot) to the immediate frame's bytes
on the real adapter, and holds the replay's instance shares to zero and the kept bytes to what the
first frame sent. Planted back — the cell ignored, the streams made every frame — two of its tests
fail. The gpu and compute arms of the corpus at 1× and 4× and `raster_golden` are the record's.
