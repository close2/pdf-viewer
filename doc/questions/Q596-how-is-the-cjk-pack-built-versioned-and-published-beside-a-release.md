# Q596 — How is the CJK pack built, versioned and published beside a release?

Source: session 1734, ADRs 2302 and 2303, `doc/questions/A569`. Asked because `doc/stack.md` has
never taken a data dependency of this size, and A569 asks that it be asked.

## The fact

The pack is five OpenType programs and the licence, about 27 MB (`doc/cjk-pack.md`), derived from
Source Han Sans 2.005R (SIL OFL 1.1) by `tools/cjk-pack.py`. The build is reproducible — two runs
from the same inputs under fontTools 4.66.1 wrote identical bytes — and the converter carries each
program's SHA-256, refusing bytes that differ. Nothing about the pack is in the repository but the
script, its inputs' digests, the licence text and the five output digests.

## What needs the owner

1. **Where the download lives.** A release asset beside each binary (`quorra-cjk-pack-<n>.zip`), or
   a separate repository whose tags the binary's digests name.
2. **Who builds it.** CI, from the four Source Han files fetched by digest, or a person once per
   pack version, with CI checking the digests only. fontTools is then a build-time dependency of a
   release, which `doc/stack.md` has never had.
3. **How a pack version follows a binary.** The digests are compiled in, so a new Source Han release
   or a change to the builder is a new binary and a new pack together; an older pack beside a newer
   binary is refused as not this pack.

## Recommendation

A release asset built by CI (1 and 2), with `SHA256SUMS` beside it and the digests in
`pack.rs` checked against it by a test that reads a built pack; one pack version per Source Han
release, renumbered only when the builder changes. fontTools stays out of every Cargo build.

## What the tree does meanwhile

The pack is built by hand into a scratch directory, and the census reads it through
`QUORRA_CJK_PACK`. The converter looks beside its executable and at `--cjk-pack`, and refuses a
collection's font as the pack's absence anywhere else. Nothing is published.
