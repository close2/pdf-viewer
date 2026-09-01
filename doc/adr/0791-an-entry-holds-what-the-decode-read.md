# ADR 0791 — An entry holds what the decode read: the raster cache's key is the `/ColorSpace` entry, and the budget charges it

Status: accepted. Session 869.
Clauses: ISO 32000-2 §7.8.3 Table 34, §8.6.5.1, §8.6.5.6, §8.9.5 Table 87.
Code: `crates/pdf-model/src/image.rs` (`RasterCache::parts`, `Cached`, `footprint`,
`RasterCache::held`).
Tests: `crates/pdf-model/tests/image_reuse.rs::a_raster_is_shared_across_resource_dictionaries_that_agree_on_their_colour_spaces`,
`::an_entry_is_charged_for_the_colour_spaces_it_holds`, and the ten that were there.
Takes `doc/todo/17`, which is deleted; ADR 0798 is the measurement that wrote it.

## Context

ADR 0798 found the 2026-09-01 soft lockup was one document met 192 at a time, and named the
document's cost to the line: `GHOSTSCRIPT-688117-0.zip-0.pdf`, a Letter page whose producer wrote
a picture as 10 260 image XObjects one sample tall, costs 10.59 GiB to *interpret* — at any scale,
with no raster at all — and `valgrind --tool=massif` put 82 % of that on
`BTreeMap<Name, Object>::clone` under `RasterCache::parts`. The cache's miss path cloned the page's
whole resource dictionary into every entry, so that a later hit could be answered only under the
same resources; the page's dictionary names all 10 260 XObjects, so each clone was about a
mebibyte; and `RASTER_BUDGET` was charged `parts.bytes()`, which for a two-by-one image is eight
bytes. Ten thousand eight-byte rasters holding ten thousand mebibyte dictionaries, under a 64 MiB
budget that could see 80 KB of it. `doc/todo/12`'s shape: a bound on the samples taken as a bound
on the entry.

`doc/todo/17` priced the fix three ways — hold what the decode read; failing that, charge the
clone; and an identity fast path in front of either. This round took the first, in a form that
makes the second unnecessary and the third already true.

## What the decode reads of the resource dictionary

The cache's doc comment said the dictionary was in the key "because §8.6.5.1 resolves a colour
space named `/CS0` through it and §8.6.5.6's `/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK` reach
even the device names". Both of those lookups are inside **one** of §7.8.3 Table 34's eight
entries. `ColorSpace` is "[a] dictionary that maps each resource name to either the name of a
device-dependent colour space or an array describing a colour space", and §8.6.5.6 puts the
defaults in the `ColorSpace` subdictionary of the current resource dictionary. In the code the
whole of it is two lines of `colour.rs` — `by_name` and `named_default`, each
`document.get_key(resources, "ColorSpace")` — and nothing else under `decode_parts` reads a key
off the resources: the masks, the matte, the explicit and colour-key masks all go through the same
colour-space parse. `grep -n 'get_key(resources' crates/pdf-model/src/image.rs
crates/pdf-model/src/colour.rs` is the check, and it prints those two lines.

So an entry that holds the `/ColorSpace` entry — the raw object, a reference usually and a direct
dictionary at worst, or `Null` where there is none — holds everything the decode read of the
dictionary, and comparing it by value is the same claim the dictionary's comparison made: two
equal entries name the same objects of the same document, so the lookups agree.

## Decision

**The decode is handed §7.8.3 Table 34's `/ColorSpace` entry and nothing else of the resource
dictionary, the entry holds that one object, and the budget charges it.**

- `RasterCache::parts` builds a one-entry `Dictionary` holding the caller's `/ColorSpace` object
  (a clone of a reference, usually) and calls `decode_parts` with *that*. This is what makes the
  claim above structural rather than a reading of `colour.rs` that decays: were a later round to
  make the decode read `/Pattern` or `/XObject` off the resources, it would find nothing there on
  a miss and nothing on a hit alike, and `tests/image_reuse.rs`'s comparison of a cached answer
  against a fresh decode under the whole dictionary would say so. The object is taken back out of
  the dictionary afterwards rather than cloned twice.
- The probe compares the caller's `/ColorSpace` object against the entry's, after the three cheap
  components, as before. For a page whose resources name the subdictionary by reference — the
  usual shape — that comparison is one `ObjectId`; the third of `doc/todo/17`'s constructions,
  keying on identity, is therefore already what happens and needs no fast path of its own.
- `Cached::bytes` is `parts.bytes()` plus `footprint(&colour_spaces)`: the object's own size and
  what it owns on the heap, walked. A name's bytes and a stream's data are over-charged — both may
  be shared with the parser's own copy — in the direction `Parts::bytes` already chooses for a
  deferred mask: over-charging bounds, and under-charging is the defect this ADR ends. A direct
  `/ColorSpace` dictionary of ten thousand names is therefore charged its ten thousand names, and
  ten thousand entries holding one evict each other under the same 64 MiB, which is the second of
  the three constructions arriving for free.
- `RasterCache::held()` is public and says what the cache currently charges, so a test can assert
  the charge rather than infer it from an eviction.

**Why not the second construction alone.** Charging the dictionary clone bounds the memory and
keeps the clone: on the witness the budget then evicts every entry as it arrives and every `Do`
decodes afresh, which `doc/todo/17` rightly priced as nothing for two-byte images — but a page
with a large resource dictionary and a large photograph drawn nine times, which is the shape
`RASTER_BUDGET`'s table was derived from, would hold a mebibyte of key beside each 26 MB raster
and have that much less room. Holding what was read costs the key nothing on every page that
names its colour spaces by reference, and is the only one of the three that makes the entry
weigh what it uses.

## What was measured

Everything below is one binary per arm, built in this tree's `gates` profile from the merged
`main` before and after the change, both beside the same `pdf-sandbox-worker` — the *before*
binary was first copied into the scratchpad, away from the worker, and `issue12213.pdf`'s JPEG
2000 image was refused there and drawn beside it, which read as a difference in the change until
the arm was re-taken (trap 10a's shape, and the digest's own header warns of it).

| the witness, page one at scale 1.0 | before | after |
|---|---|---|
| `ru_maxrss` of the process (`getrusage`, exact) | **10.59 GiB** (11 105 496 kB) | **0.15 GiB** (157 816 kB) |
| wall clock | 5.4 s | 0.7 s |
| `tools/bounded.sh`'s once-a-second sample of the tree | 10.60 GiB under `--data 16` | 0.02 GiB under `--data 2` — a run shorter than its sampling interval, which ADR 0798 says the sampler cannot see |

- **The page is the same page.** `render_at` at scale 1.0 on the witness, `22060_A1_01_Plans.pdf`
  (the cache's own witness, four 26 MB rasters drawn nine times each), `issue16263.pdf` (one `Do`
  forty times over a packed mask) and `issue12213.pdf` (JPEG 2000 through the worker): all four
  PNGs byte-identical between the arms.
- **Every display list is the same list.** `examples/display_list_digest` over the 975 first pages
  of `doc/pdf.js/test/pdfs`, both arms beside the worker: the two files are identical.
- **The two new tests fail against the old code**, run against it with a `held()` shim so that
  the second compiles: the sharing test's second `Do` decodes again under a dictionary that
  differs only in ten thousand `/XObject` names, and the charge test finds both caches holding
  sixteen bytes.
- **The re-walk.** ADR 0798's arm, repeated after the change: documents 341–680 of
  `batch2/GHOSTSCRIPT` in C-locale sorted order (the witness is the 153rd of them), one at a time
  under `tools/bounded.sh --data 2 -- render_at <doc> 1 1.0 …`. **Nothing ran out**: 333 of the
  340 exit 0 with a worst sampled peak of 0.03 GiB, and the 7 that abort are `render_at`'s own
  `expect`s — 6 documents that want a password and 1 with no page one, exactly the survey's `6
  locked, 1 pageless` — with the wrapper saying so rather than naming the bound (trap 11, as 866
  wrote it). Then the same 340 as **one** survey shard through the wrapper, 24 rayon threads,
  32 GiB: **peak 1.55 GiB resident** in 3 s, where 866's table has the same half of the same
  slice at **12.58 GiB**. `doc/todo/03` §38 carries the line beside 866's.

## Consequences

- `doc/todo/17` is deleted; this ADR carries its argument. ADR 0798 and the eight-hundred-and-
  sixty-sixth round's history file go on citing it, as the todo directory's rule expects.
- §7.8.3's and §8.9.5's ledger rows say what the key holds now and why.
- The cache's key is now four things and a fifth that names the stream: the stream, §7.8.3 Table
  34's `/ColorSpace` entry, the fill colour and the conversion. `tests/image_reuse.rs`'s table is
  updated to say so, with one test per direction for the second.
- **The witness is the direct shape, and it is what the charge is for.** Its page writes the
  `/ColorSpace` subdictionary direct, 1284 names in 21 KB of file, beside the 10 256 `/XObject`
  names in 190 KB that made the old clone a mebibyte. So each entry now holds a copy of those
  1284 name-to-reference pairs — `footprint` charges it about 90 KB — and 64 MiB holds some seven
  hundred of them before the eviction arithmetic takes the oldest. That is `doc/todo/17`'s
  second construction happening inside its first, on exactly the page it was priced for, and its
  price there is what that file said: every one of the 10 260 images decodes once, because none
  is drawn twice, and the peak is 0.15 GiB. A page that names its subdictionary by reference pays an
  `ObjectId` per entry instead; both shapes are ordinary — a byte search over `doc/pdf.js` finds
  90 documents writing `/ColorSpace <<` and 75 writing `/ColorSpace N 0 R`, an undercount on
  both sides for object streams — and what a direct one costs is its *names*, which is small
  everywhere but under a producer that wrote one colour space per strip. **The narrowing left**, if a page is ever found that pays for a direct
  subdictionary and draws its images twice, is to hold the names the decode *resolved* rather than
  the subdictionary they were resolved in — a recorder threaded through `colour::ColourSpace::parse`,
  which this round judged more API than the witness earns.
