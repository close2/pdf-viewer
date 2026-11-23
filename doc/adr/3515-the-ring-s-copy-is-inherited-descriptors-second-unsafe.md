# 3515 — The ring's copy is `inherited-descriptors`' second `unsafe`

Session 2203. Status: accepted. Context: ADRs 1945, 3361, 3513; `doc/questions/A979` (one shared
crate, "so future confined helpers do not widen the list again"); `viewer-qt/tests/unsafe_position.rs`;
`doc/questions/Q1024` asks about the crate's name.

## Decision

1. **The mapping is `memmap2`'s raw one**, whose construction is safe because it hands out a pointer
   and never a slice. What needs `unsafe` is going through the pointer, and only that.
2. **It lives in `inherited_descriptors::shared_mapping`**, the one crate the tree's sweep names for
   a confined helper's `unsafe` — so `confined-transport`, the witness and every helper keep
   `forbid(unsafe_code)`, and the sweep's list stays five names. Linux-only, as `memfd` is.
3. **Two lifts, asymmetric on purpose.**
   - `Reader::copy_out` (the host): a `copy_nonoverlapping` from the read-only mapping into the host's
     own memory. The worker can write the object at any moment, so the host never forms a `&[u8]`
     into it; a copy reads bytes, `u8` has no invalid value, and a racing write changes only which
     bytes arrive.
   - `Writer::region_mut` (the worker): a `&mut [u8]` of a slot. Sound because the only other
     mapping is the host's and it is read-only, and the host writes the object by no other route.
4. **The callers are pinned by test**, as the close's are: `tests/callers.rs` now holds the module
   to two lifts, each on one named function with one `unsafe`, and `confined-transport/src/ring.rs`
   as the only other source naming the crate.

## The cost, written down

The crate's name says what its first function does and not what its second does. A979 asked for
one shared home so the list would not grow; a sixth name would grow it, and a new crate would need
the owner's word. Q1024 asks whether to rename the crate (a confined helper's `unsafe`, not its
inherited descriptors) or to name a separate crate in the sweep. Until answered, the description in
its `Cargo.toml` and its module documentation say both things it holds.
