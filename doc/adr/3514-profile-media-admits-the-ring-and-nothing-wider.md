# 3514 — `Profile::Media` admits the ring and nothing wider

Session 2203. Status: accepted. Context: RFC 0015 section 4.3 ("adds `mmap` of one passed
descriptor to `Profile::Media`'s list, narrowed by argument as `Profile::Interpreter`'s `fcntl` is")
and section 6 ("the one widening of a profile this plan asks for"); ADRs 0888, 1608, 3297, 3361,
3513.

## Decision

`Profile::Media` stops sharing `Profile::Script`'s rules exactly. `Profile::Script` is unchanged.

1. **Descriptors: one slot.** `RLIMIT_NOFILE` is 4 (`MEDIA_DESCRIPTOR_LIMIT`), so the only number a
   new descriptor can take is 3 (`MEDIA_RING_DESCRIPTOR`, held equal by a `const` assertion), and
   nothing on the list makes a descriptor: it is filled only by one received beside a frame.
2. **`recvmsg` on standard input only** — argument 0 equal to 0.
3. **`mmap` in two rules**: the ring's — protection exactly `PROT_READ | PROT_WRITE`, flags masked by
   the mapping type and `MAP_ANONYMOUS` equal to `MAP_SHARED`, descriptor 3 — and the allocator's —
   protection without `PROT_EXEC`, flags so masked equal to `MAP_PRIVATE | MAP_ANONYMOUS`. That is
   narrower than the script rule it replaces for this profile, which admitted any mapping type.
   `mprotect` stays absent, so nothing the worker maps is ever executable.
4. **No `close`, `fcntl` or `fstat`.** The worker keeps the ring's descriptor for its life
   (`RingWriter::attach` releases it from its owner, so nothing closes it on the way out), the ring's
   frame states its size, and so the one slot stays full: **a second descriptor is dropped by the
   kernel with `MSG_CTRUNC`**, not refused by the worker's good conduct.

ADR 3361's rule holds at start: the witness and `pdf-media-worker` still call
`keep_only_the_standard_descriptors` first, so slot 3 is empty when the confinement is applied.

## Consequences

- `pdf-media-worker`'s audio path reads with `read` and is handed nothing; it gains no capability it
  uses. Its tests pass unchanged.
- `madvise` was already admitted, so a worker can discard its own pages of the ring and the host
  reads zeros — a frame it could have sent.
- `pdf-sandbox`'s `a_confined_media_worker_maps_its_ring_and_nothing_wider` holds each edge: the ring
  maps shared and read-write; executable or read-only is the filter's `SIGSYS`; a second descriptor
  never arrives. Its children run with `MALLOC_ARENA_MAX=1`, because the harness thread's arena
  reserves more address space than the profile's ceiling, and every mapping after the confinement
  would otherwise fail for want of room — a refusal that is not the filter's (trap 13). Planting
  `PROT_EXEC` back into the ring's rule, or a ceiling of five, fails it.
- The probe for an executable mapping of the ring holds that test file's one `unsafe` (memmap2's
  `map_exec`), because no safe call asks for one; the mapping is never touched.
