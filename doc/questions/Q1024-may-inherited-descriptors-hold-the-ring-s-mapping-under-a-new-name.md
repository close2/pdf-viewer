# Q1024 — May `inherited-descriptors` hold the ring's mapping, under a name that says so?

Asked by round 2203 (ADR 3515), for video's second round (RFC 0015 section 6): the shared-memory
ring.

**The condition.** The ring needs `unsafe` in two places — the host copying a slot out of its
read-only mapping through a pointer, and the worker writing a slot of its read-write mapping — and
no crate that touches PDF bytes or the wire may hold it. A979 asked for one shared crate "so future
confined helpers do not widen the list again", and ADR 3361 made it `inherited-descriptors`, the one
name in `viewer-qt/tests/unsafe_position.rs` for a confined helper's `unsafe`. Round 2203 put the
mapping there, as a module `shared_mapping` beside the close, pinned by the same test (two lifts,
each on one named function; `confined-transport/src/ring.rs` its only caller), and the sweep's list
is unchanged at five names. What does not fit is the crate's *name*: it says what the close does,
and the mapping is not an inherited descriptor.

**The question.** Which do you want?

1. **Keep it as built** — one crate, one sweep entry, the name a little narrower than the contents.
2. **Rename the crate** to say what it holds — for example `confined-unsafe` or `helper-unsafe` —
   still one sweep entry, and the five helper programs, the witness and the transport renamed in
   their `use` lines and manifests.
3. **A separate crate** (`shared-ring`, say) named in the sweep as a sixth entry — one responsibility
   a crate, at the cost of the list growing, which is what A979 asked to stop.

**Recommendation**: 2 — the list stays at five as A979 wanted, and the name stops misdescribing the
crate. Nothing waits on the answer: the ring works and is tested as built.
