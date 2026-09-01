# A mebibyte per image XObject: the raster cache charges the samples and clones the resources

Status: **found in the eight-hundred-and-sixty-sixth session**, diagnosed to the line, not fixed —
the round was a measuring one beside a gating one and a change to `pdf-model` owes the whole
`doc/todo/02` §2 sequence.
Priority: 17 — a defect of the kind `CLAUDE.md`'s principle 3 names: pathological content that
exhausts memory under budgets that cannot see it. It is what took the machine into a soft lockup
on 2026-09-01 once eight shards met it at once (ADR 0798).
Witness: `corpus-cache/tika-issue-tracker/batch2/batch2/GHOSTSCRIPT/GHOSTSCRIPT-688117-0.zip-0.pdf`
— 3.2 MB, a Letter page, 11 562 objects of which 10 260 are image XObjects one sample tall and two
to nine wide. **10.59 GiB to interpret page one**, at any scale; the survey calls it `complete`.
Code: `crates/pdf-model/src/image.rs`, `RasterCache::parts` and the `Cached` entry beside it.

## The site

`valgrind --tool=massif` over `display_list_digest` on the witness: at the peak, **82 % of
11.16 GB is `BTreeMap<Name, Object>::clone`, reached from `RasterCache::parts` out of
`Interpreter::draw_image`**. That is this line of the cache's miss path:

```rust
self.entries.push(Cached {
    identity,
    stream: Arc::clone(stream),
    resources: resources.clone(),   // the page's whole resource dictionary, per image
    fill,
    into: into.clone(),
    parts: parts.clone(),
    bytes,                          // parts.bytes(): the samples, and only the samples
});
```

Every `Do` of a new image clones the resource dictionary it was drawn from, so that a later hit
can be answered only under the same resources. The page's resource dictionary here names 10 260
XObjects, so the clone is about a mebibyte, and 10 260 of them are held at once because
`RASTER_BUDGET` — 64 MiB — is charged `parts.bytes()`, which for a two-by-one image is eight bytes.
The cache is under a budget and the budget cannot see what the cache weighs. It is
[`12`](12-one-bound-two-jobs.md)'s shape one more time: a bound on the samples was taken as a bound
on the entry.

## The fix, priced

Three constructions, in the order to try them:

1. **Do not hold the dictionary; hold what the decode read from it.** `decode_parts` consults the
   resources for a handful of named lookups (the colour space by name, at most). Record the
   entries it resolved — the names and the objects they resolved to — and compare those on a hit.
   An entry then weighs what it used, which for this witness is nothing. The honest cost is that
   `decode_parts` has to report what it read, which is a small API change inside one file.
2. **Failing that, charge the clone.** `bytes` becomes the samples plus a measured size of the
   resources clone, so that ten thousand entries evict each other under the same 64 MiB. Correct
   and cheap, but the witness then decodes every image afresh on every `Do`, which is the cost the
   cache exists to remove — and on this witness that is still ten thousand two-byte decodes, which
   is nothing.
3. **Key on identity rather than content.** The resources dictionary usually arrives from one
   object; its reference would do as a key where it has one. It does not always have one (a
   direct dictionary in the page), so this is a fast path in front of 1 or 2, not a replacement.

Whichever lands: re-run the second half of the slice ADR 0798 walked
(documents 341–680 of `batch2/GHOSTSCRIPT` in sorted order)
one document at a time under `tools/bounded.sh --data 2 -- render_at …` until nothing runs out,
then `display_list_digest` on `doc/pdf.js` before and after — a cache that answers differently is
a display list that differs, and this is the instrument for saying it does not — and then the
whole of `doc/todo/02` §2.

## What it is not

Not a leak: the memory comes back when the page's interpretation is dropped, and a single-threaded
walk over 340 documents is flat outside this one. Not the rasteriser: the cost is identical with no
raster at all. Not the sandbox, the fonts or the press cache, each of which was read for the
purpose and is bounded.
