# 3513 — A stream's large items cross in a sealed ring, and the grant is the lease on its slots

Session 2203. Status: accepted. Context: RFC 0015 sections 4.3 and 6 (video's second round); RFC
0012 section 5.1 (a raster copied per reply); ADR 3501 (the streaming arm and its carriage seam);
ADRs 3514 to 3516.

## Decision

1. **`confined_transport::ring` and `Host::attach_ring`.** The host creates a `memfd`
   (`MFD_CLOEXEC | MFD_ALLOW_SEALING`) of `slots × slot_len` bytes — `ring::Shape`, at most
   `MAX_SLOTS` (16) slots, the slot sized from the stream's own frame — sizes it and seals it with
   `F_SEAL_SHRINK`, `F_SEAL_GROW` and `F_SEAL_SEAL` before anything maps it, maps it read-only, and
   sends it once beside a `RING` frame (kind `0xF8`, the shape in eight bytes) by `SCM_RIGHTS`. The
   worker maps it read-write and answers an empty `RING`; anything else ends it.
   - `F_SEAL_SHRINK` because a mapping truncated under it faults with `SIGBUS` on access and the
     host reads every slot; `F_SEAL_GROW` so that the announced shape is the object's for its life;
     `F_SEAL_SEAL` so that the set is final. Not `F_SEAL_WRITE`: the worker writes.
2. **Bounded by the worker's memory.** `Shape::new` takes the caller's budget, and `attach_ring`
   refuses a ring past half the address-space ceiling the worker's greeting reports — the other half
   is the decoder's. RFC 0015 section 4's table puts the video worker's ceiling at 1 GiB; today's
   `Profile::Media` ceiling (64 MiB) admits three 1080p RGBA slots (24.9 MB).
3. **Once a worker.** The worker keeps the descriptor (ADR 3514), so a second ring cannot reach it:
   `attach_ring` answers `RingError::Attached`, and a stream that would not fit asks for a new worker.
4. **The carriage.** `Carriage::Slot` (byte 1): the item's body is a slot and a length, four bytes
   each (`stream::Body::Slot`). `Carriage::Inline` stays, for small items; the producer chooses per
   item (`Emitter::emit` or `Emitter::emit_in_ring`).
5. **The ownership rule: the grant is a lease.** Item `q` is written into slot `q mod slots`. A
   slot is the worker's from the grant covering the item it carries until that item arrives, and the
   host's at every other moment. The host clamps every grant to the slot count
   (`Reception::with_ring`), so one grant's items name distinct slots; it copies a slot out in the
   step that received its item and writes the next grant in the step after, and ADR 3501 already
   grants only once every item of the last grant has arrived. The worker's `emit_in_ring` reads the
   grant first and hands its closure the slot second, so a stopped stream's slot is never touched.
   An item naming any other slot, or a length past a slot, is a broken contract and ends the
   worker; its bytes count against the stream's bytes budget before the slot is read.
6. **A write without a message is not seen, and costs nothing.** A worker that scribbles on a slot
   it does not hold is invisible — a write is not a frame — and it can only change the bytes of the
   frame the host is copying, which it could have sent: the host copies through a pointer and never
   holds a reference into the mapping (ADR 3515).

## Consequences

- The control frames and their rules are ADR 3501's unchanged; the ring adds one frame kind, one
  carriage and one clamp.
- The host's memory for a ring stream is the ring plus one item's copy; `next_item` still returns an
  owned body, so a caller's code reads the same for both carriages.
- Pacing is still one grant per round trip; a host that wants the worker never idle needs more
  slots than its window, which the clamp allows and nothing yet asks for.
