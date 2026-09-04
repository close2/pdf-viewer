# 0887 — The cache is right to refuse, and the instrument's bound comes off

Session 923. Status: **accepted**. The second of this round's two records: the two questions ADR
0886's measurement freed, answered separately because they were tangled together for two sessions.

## Context

Three things were named as one defect in `doc/todo/58` §5 and ADR 0878: a cache that refuses the
entries most expensive to recompute, a directory whose listing costs an extraction per entry, and a
corpus walk bounded to four entries a directory to get past both. ADR 0886 measured the second and
found it to be neither the cache's doing nor the layout's. That leaves the other two, and each has
an answer of its own.

## 1. `Cache::put`'s refusal of an oversized entry is kept, and the argument is not "it did no harm here"

`Cache::put` returns early — answering the caller, storing nothing — for an entry larger than the
whole budget. Round 911 found that and ADR 0865 §3 recorded it as the reason a test passed without
its fix; sessions since have read it as a defect on its face, on the ground that a cache which
refuses what is most expensive to recompute has its policy backwards.

**It does not have its policy backwards, and the reason is what the alternative costs.** Admitting
an entry larger than the budget means evicting *everything else* to hold one item — which is the
textbook cache-pollution failure, and on a mount it is a specific one: a `cp` of a single 100 MiB
render would throw away the working set of every other directory a file manager has open, to hold
bytes whose reader already has them. Three facts make the refusal cheap enough to keep:

- **The `stat` half of the cost is already gone.** ADR 0865 §3's size note is taken *before* the
  early return, so a file too large to store is still `stat`ed for free after the first time. What
  the refusal costs is a re-*read*, not a re-`stat`.
- **A reader in flight holds the bytes.** `Vfs::open` hands back a `Handle` over an `Arc<[u8]>`, so
  the whole of one `cp` — the `stat`, the `open` and every `read` from the handle — is one
  generation whether or not the cache kept a copy.
- **It is a memory budget rather than a guard**, which the module comment already says: refusing
  to *answer* would make the budget a limit on what the mount can serve, and that distinction is
  `doc/todo/10`'s. The bomb guard is `pdf_syntax::Limits` inside the worker.

The four alternatives were considered and each is worse for this tree: admit-and-evict is the
pollution above; a singleton tier is the same thing with a second budget to tune and no witness
asking for it; keeping the derivation rather than the bytes is what the *worker* already is, since
every generation is a `pdf_transform` plan re-run from a document that has not changed; and a disk
half is RFC 0003 §5.5's optional second budget, which is worth building when something measures a
mount thrashing rather than because this question came up.

**What was actually missing from the cache was a third kind of entry, not a fourth policy** — a
directory's own names, which ADR 0886 added. Both notes have the same shape and the same
justification: they are derived from a run that happened, they are bounded by the document rather
than by a number, and they are outside the byte budget because neither is content.

So this is a decision to change nothing, written down because "the cache refuses the entries most
expensive to recompute" had been repeated three times without anybody pricing the alternative.

## 2. The walk's bound comes off

`tests/read_corpus.rs`'s `ENTRIES_SAMPLED = 4` was session 919's way past the twenty-five minutes,
and ADR 0878 was explicit that it bounded the *instrument*: "the two bounds are the honest limit …
what would remove the bound is not a bigger budget but a generator that can state a length without
writing it". A walk that skips the pathological case cannot see the next one.

With ADR 0886's fix that document's whole ten-thousand-entry directory is listed, `stat`ed and read
in a quarter of a second, so the bound has no reason left and is removed: `Bounds` carries `pages`
alone, `attachments()` no longer takes it, and the `entries listed and not read` column is gone
from the run's own printout because it is now always zero. Listings were always whole; what comes
back is reading every entry of `images/NNNN/` and of `attachments/` on a widened document.

**And the walk got faster while reading more, which was not the expected result.** Session 919's
run: 1132 documents in 315.5 s, 20 976 entries `stat`ed, 14 274 files read (784.8 MiB). This
round's, same population, no entry bound: **1132 documents in 162.9 s, 31 435 entries `stat`ed,
24 733 files read (792.6 MiB)** — half the wall clock for half again as many questions, with every
column of the comparison still zero. The four-entry bound was capping the *count*, and what the
count cost was the extraction it re-ran.

`PAGES_SAMPLED` stays. It bounds a different cost — pages extracted, drawn twice and read — which
nothing in this round touched, and ADR 0878's reason for it is unchanged.

## 3. Where a fix like this belongs, since the round was asked

Three places could have taken it, and only one is right:

- **The layout** — `images/` as a directory per page is session 899's departure from RFC 0003 §4
  and `doc/questions/Q14` asks the owner to ratify it. It is *not* the problem: a per-page listing
  costs one extraction of one page, and the flat directory the RFC proposed would have cost one
  extraction of the whole document. Nothing here is a reason to revisit it, and the addition to
  `Q14` says so rather than leaving the question standing under a defect that was never its.
- **What a `stat` is allowed to cost** — RFC 0003 §5.5 forbids the tempting answer outright: a
  size that was estimated truncates the file for every reader (the ffmpegfs lesson). A `stat` that
  generates is the rule, and the note that makes the *second* one free is the only relaxation
  available.
- **The core**, which is where it went. The cost was a question the core asked itself, in a
  function neither face can see.

## Consequences

- One `#[test]` that counts a generator's runs rather than a tree's productions, and one section of
  `examples/vfs_cost` that prints the clock a gate would ratchet. There is still **no perf floor on
  this crate** — `doc/todo/58` §5 now says that is the sharpest thing missing, because a
  hundredfold regression lived here for four sessions with the whole gate sequence green.
- ADR 0878's account of what one document cost stands as a measurement and is wrong as a diagnosis;
  ADR 0886 says so, and this file does not repeat it.
