# 0886 — A name validated by re-running the run that named it

Session 923. Status: **accepted**. The first of this round's two records: what actually cost
twenty-five minutes on one document, which is not what three documents said it was.

## Context

`doc/todo/58` §5 and ADR 0878 both carried the same sentence about
`corpus-cache/tika-issue-tracker/batch1/PDFBOX/PDFBOX-186-0.pdf`, which states **10 084 images on
one page** so that `/images/0001/` is a directory of ten thousand files:

> A `stat` generates (RFC 0003 section 5.5); a read puts a whole extraction run in the cache at
> once; and **a run too large for the cache's budget is put nowhere at all**, which is round 911's
> own finding about `Cache::put` (ADR 0865 §3). So each of those twenty thousand questions re-ran
> an extraction of ten thousand images.

Session 919's corpus walk was inside that one document for twenty-five minutes, twice, and bounded
itself to four entries a directory to get past it (ADR 0878). This round was asked to separate
three things — the cache's admission policy, the layout, and the instrument's bound — and to
measure before and after.

## The measurement came first, and it disagreed with the diagnosis

`crates/pdf-vfs/examples/vfs_cost.rs`'s new section, over that document, in process:

```
list /images/0001: 10081 entries in 0.33 s
list again:        10081 entries in 0.20 s
  [   0] 01.jpg     4277 B  stat 547.066 ms  read 176.712 ms  generated 1
  [   1] 02.jpg      352 B  stat 177.528 ms  read 179.320 ms  generated 1
  [   2] 03.jpg      352 B  stat 178.338 ms  read 176.071 ms  generated 1
  …
512 entries stat'd and read in 274 s
```

Three things in that output contradict the recorded reason:

- **The outputs are 352 bytes.** They are nowhere near `Config::cache_bytes`'s 64 MiB, so
  `Cache::put`'s early return for an entry larger than the whole budget never fires on them. The
  whole run is 3.4 MiB and fits with room to spare.
- **`Vfs::generated` is 1 and stays 1.** The tree produced the bytes once. Every read after the
  first came out of the cache, exactly as designed.
- **And each question still cost 176 ms**, which is what a `pdf_transform::images` run over that
  page costs — `vfs_cost` prints it as "a page's images 173.70 ms in process".

So the extraction *was* running twenty thousand times, and it was not the cache doing it.

## Decision

**It was the path validation.** `pdf_vfs::locate_in`'s `ExtractedImage` arm read:

```rust
if !spells(current, path, page) || !images(current, page)?.contains_key(&name) {
    return Err(missing());
}
```

`images()` is the extraction. Every `stat`, every `open` and every `list` of a path under
`images/NNNN/` goes through `locate_in` first, so **the name was checked by re-running the run that
produced it** — and that is one extraction per *question* rather than per run, whatever the cache
holds. `Vfs::entries_of`'s `ImageInventory` arm did the same for the listing.

The reason it was written that way is a good one and survives: RFC 0003 §4's `images/NNNN/` is the
one directory in the layout whose listing *is* an extraction's own output names (session 899's
departure, `doc/questions/Q14`), because an output's name depends on its codec and on whether a
mask travels beside it — so a listing that predicted names would name files a read cannot produce.
A listing and a read that are one call cannot disagree. What did not follow is that the call has to
be *made* every time.

**So the cache gets the second kind of entry `doc/todo/58` §5 already named** — "[c]aching the
listing itself is a second kind of entry the cache does not have":

- `Cache::note_inventory` / `Cache::inventory` hold **a directory's own names**, keyed by
  (generation, path) exactly as the bytes and the sizes are, and outliving the eviction of the
  bytes exactly as the sizes do (ADR 0865 §3).
- They are **names rather than content**, so they are outside the byte budget and bounded by the
  document: one entry per directory per generation, dropped by `Cache::retain` with everything
  else when the generation changes.
- They are **a run's own outputs rather than a guess**, which is the same rule RFC 0003 §5.5
  states for a size and for the same reason: an invented name is a file that cannot be read.
- `extract_images` is the one place that runs the extraction, puts every output's bytes in the
  cache and notes the names; `image_names` answers from the note where there is one. `locate_in`,
  `entries_of` and `generate` all go through those two.

A consequence worth stating rather than discovering: **a listing now warms the reads.** Before,
only a read put the bytes in the cache; now the listing that names the directory has already
extracted it, so `ls` followed by `cp -r` is one run rather than two.

## What it is worth

| on `PDFBOX-186-0.pdf`, `/images/0001` (10 081 entries) | before | after |
|---|---|---|
| the listing | 0.33 s | 0.25 s |
| the same listing again | 0.20 s | 0.31 ms |
| one `stat`, one `open` | 176 ms each | 0.010 ms each |
| 512 entries `stat`ed and read | **274 s**, measured | — |
| all 10 081 `stat`ed and read | ≈ 90 min, extrapolated from the 512 | **0.25 s**, measured |

And on an ordinary document — `doc/Tagged-PDF-Best-Practice-Guide.pdf` page 60, two images — the
first `stat` was 53 ms and every question after it 17 ms; they are 4 µs and 2 µs now.

## Trap 13: the gate was run against the defect

`tests/a_face.rs`'s `a_directorys_names_are_asked_of_the_extraction_once` counts the
`Query::ExtractImages` questions a mount puts to its worker while listing `images/0035/`, `stat`ing
and reading every entry, and listing it again. It asserts one. With the two lines above restored it
fails, **6 against 1**, on the committed document's single-image page; with them removed it passes.

**A `Vfs::generated` assertion could not have caught this, and that is the lesson.** That counter
says how many virtual files the tree produced the bytes of — the right question for ADR 0865 §3's
size notes, and blind to a cost paid in *validating a name* rather than in producing bytes. It read
1 throughout. A property about how often a generator runs has to count the generator running.

## Consequences

- `doc/todo/58` §5's two entries about this are closed, with the numbers, and the sentence that
  blamed `Cache::put` is struck rather than quietly edited: it was in two documents and an ADR, and
  a wrong reason that reads plausibly is worse than no reason.
- `doc/questions/Q14`'s recommendation is strengthened rather than weakened — the per-page layout
  is what kept the *listing* one extraction, and the cost was in a place the layout does not reach.
  The question stays open and the addition is recorded there.
- ADR 0887 covers the cache's admission rule, examined and kept, and the walk's bound, taken off.
