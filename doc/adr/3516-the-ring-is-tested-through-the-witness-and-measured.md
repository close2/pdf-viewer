# 3516 — The ring is tested through the witness, fuzzed at its carriage, and measured

Session 2203. Status: accepted. Context: ADRs 3502, 3513 to 3515.

## Decision

- **The witness** (`tools/stream-witness`) now reads with `confined_transport::link::Link`, so a
  descriptor beside a frame arrives, attaches a ring on `RING`, and answers a second `RING` with the
  number of descriptors that came with it. Three modes: `Ring` (an item past 4 KiB in its slot, a
  smaller one inline, each byte item `q`'s low byte), `RingWrongSlot` and `InlineFilled` (`Ring`'s
  items, all inline). Tests: ten items through three slots arrive whole and in order; small items
  inline beside the ring; a slot not held ends the worker; a second ring refused by the host and its
  descriptor by the kernel; a ring past half the worker's address space refused; bytes and frames
  budgets through the ring, the worker kept; a host dropped mid-stream ends the worker before its
  own mapping goes. Unit tests hold the clamp, the turn of the slots, the three slot refusals and
  the byte count; `ring.rs` holds the shape and the seals.
- **Fuzz**: `confined_stream`'s window byte's top bit attaches a ring of up to sixteen 64-byte
  slots; it decodes `Shape` and items directly, and asserts a grant within the slots and a slot item
  handed on naming `q mod slots` and fitting it.
- **Measured** (`the_copy_the_ring_saves_measured`, ignored, printed): 120 frames of 8 294 400 bytes,
  window 3, each frame written afresh by the worker; dev profile, this VM under a load average of 29
  to 50 on 32 CPUs. Three runs: inline 28.7, 35.9, 29.6 frames per second; ring 159.3, 227.2, 189.9.
  The pipe's copy in and out of the kernel, in 64 KiB turns, is what the ring removes; the host's
  one copy out of its slot remains.

## Other platforms

The ring is Linux's. `Host::attach_ring` answers `RingError::Unsupported` elsewhere and every item
crosses inline, as the transport's platform code already chooses for descriptors (Windows's channel
is a pipe; `channel_pipe.rs`). A Windows ring would be its own shared section passed its own way,
argued with that platform's confinement (`doc/todo/35`). The slot carriage decodes everywhere; with
no ring, a slot item is a broken contract.
