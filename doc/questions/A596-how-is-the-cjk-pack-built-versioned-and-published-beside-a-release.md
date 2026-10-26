Status: complete
Given: 2026-10-26, in conversation — transcribed by the round
Owes: the publication — the pack released as an asset beside each binary, built by CI from the
four Source Han files fetched by digest: `quorra-cjk-pack-<n>.zip` with `SHA256SUMS` beside it, a
test reading a built pack to check the digests compiled into `pack.rs`, one pack version per
Source Han release renumbered only when the builder changes, and fontTools a build-time
dependency of the release, never of a Cargo build; the digests stay compiled in, so an older
pack beside a newer binary is refused as not this pack (acting round)

> agreed on all

Reading: one word answered the three questions presented together, and it adopts each
recommendation as presented. This one is Q596's own, quoted and attributed: *"A release asset
built by CI (1 and 2), with `SHA256SUMS` beside it and the digests in `pack.rs` checked against
it by a test that reads a built pack; one pack version per Source Han release, renumbered only
when the builder changes. fontTools stays out of every Cargo build."*

What the answer settles:

- **Where the download lives: a release asset beside each binary**, not a separate repository —
  the owner's own two-downloads ruling (A569) answered in its mechanics: the base program keeps
  its size, and the tens of megabytes sit beside the exe as one file a user who converts CJK
  fetches once.
- **Who builds it: CI, from the Source Han files fetched by digest.** The build is reproducible
  (two runs under fontTools 4.66.1 wrote identical bytes), so the machine that builds it does not
  add a trust question; fontTools enters at the release, which is `doc/stack.md`'s first
  build-time dependency of a release and stays out of every Cargo build.
- **How a version follows a binary: through the digests.** They are compiled in, so a new Source
  Han release or a builder change is a new binary and a new pack together, and an older pack
  beside a newer binary is refused as not this pack — the pack and the program cannot drift
  apart silently, which is what the reproducible build and the refused-bytes rule were for.