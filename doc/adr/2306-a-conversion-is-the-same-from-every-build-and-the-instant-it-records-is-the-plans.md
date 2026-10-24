# 2306 — A conversion is the same from every build, and the instant it records is the plan's

Session 1736, the archive slot of batch one hundred and thirteen. Status: implemented. Context:
`doc/history/1734` ("Found, not fixed": `143424271191706722106594310729442158189.pdf` refused by two
builds of one source and converted by two later ones; a replay test that failed once under load with
a different trailer `/ID`); ADR 2295 (the property cut past the findings cap); RFC 0002 section 9.

## 1. The build-dependent conversion was a source difference

The brief's hypothesis was an order the hasher's seed decides. It did not hold. The archive path
holds no `HashMap`, `HashSet` or `RandomState` (`pdf-archive/src`, `pdf-transform/src/archive`), no
map keyed by an address and no thread whose order reaches the output. Measured: ten conversions from
one release build of HEAD and ten from a second build in a target directory of its own (the two
binaries' digests differ) gave byte-identical files apart from the `stEvt:when` the removal records
and the `/ID` that digests it, each removing 89 properties and converting at 2b. The refusal is
reproduced exactly by the same source with ADR 2295's `schema_packets` hunk taken back: 35
properties removed, the output refused at `metadata/properties-use-known-schemas`. Round 1730 made
that hunk in the same batch and saw the same document move (`doc/corpora` at 2b, one more converted);
round 1734's first builds were of the batch's starting source and its later ones held round 1730's
hunk, which its record called a move "not mine". Nothing is fixed for it, because nothing is wrong.

## 2. The replay's `/ID` was the clock

ISO 19005-2 section 6.6.6 asks each recorded action for when it happened, and §14.4's identifier is
this writer's digest of every byte written, that instant's included. Two conversions a test runs back
to back straddle a second boundary now and then, and a loaded machine stretches a conversion and makes
it likelier; the test blanked the instant and not the digest of it. **The instant is now an input**:
`ArchivePlan::clock` is `Clock::System` (the machine's clock, read once, when the packet is written)
or `Clock::At` (the caller's instant). The replay test states one and compares whole files, the
instant and the identifier included, and checks the instant written is the one stated. The command
line passes `Clock::System`; a flag for an archive reproducible from the shell (the
`SOURCE_DATE_EPOCH` convention) is not built, and nothing asks for it yet. The cost is one field on
each of the fourteen `ArchivePlan` literals.

## 3. The two-seed test

std's `RandomState` draws its keys once per thread, so `a_conversion_is_the_same_bytes_under_two_hasher_seeds`
converts one fixture on two fresh threads and compares the bytes: forty metadata streams each state a
date of the XMP basic schema as a sentence — more places than the findings keep, so the cut walks every
packet — and the removal is recorded at a stated instant. A `HashMap` planted to order the removed
properties makes it fail (trap 13), and it passes without the plant.
