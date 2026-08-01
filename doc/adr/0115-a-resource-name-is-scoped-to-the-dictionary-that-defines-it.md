# ADR 0115 — A resource name is scoped to the dictionary that defines it

Status: accepted, 2026-08-01.

## How it was found

Not by a report, and not by a picture. The handover's standing item — "ask whether the
*decompression* can be avoided rather than made faster", 28% of interpretation being
`zlib_rs` — was measured by counting every call to `Document::decoded_stream_data` on the
benchmark page. Ten inflations per interpretation, ~300 KB of compressed input, and one stream
of 88 501 bytes inflated **twice**. Its dictionary has `/Filter /Length /Length1`: an embedded
font program.

Chasing why a font file is read twice led to `Interpreter::font`, and to this:

```rust
self.load_font(FontKey::Named(name.to_owned()), dict.as_ref(), name)
```

## The defect

§8.10.1 gives a form `XObject` a `/Resources` entry of its own, and §7.8.3 makes a resource name
mean whatever the *enclosing* dictionary says it means. A page's `/F1` and a form's `/F1` are two
fonts as often as they are one.

The cache was keyed by the name. So the second `/F1` was never loaded: the first one's glyphs
were returned, with **nothing reported**. That is trap 1's archetype word for word — "the font
loaded, nothing was reported, the wrong glyphs were drawn" — and it had been in the tree since
the font cache was added, thirty-one sessions ago.

The fix is the key: a font is cached under its dictionary's object identity, which is what
`shading::Cache` has done since the seventy-third session for exactly this reason. The comment
on `resource_entry` — "a cache keyed by identity wants the name of it, and resolving first
throws that away" — already said what to do; the font path simply was not one of its callers.

A resource dictionary that states its font *directly* rather than by reference has no identity
to key on and is loaded afresh each time. Correctness before speed, and the case is unreached:
every one of the 974 corpus documents states its fonts indirectly.

## Consequences, measured

**Documents drawing incompletely rises 89 → 91, and it is trap 5's rise.** Two documents were
drawing another font's glyphs in silence and now say what they actually name:

- `issue17492.pdf` names a `/Helvetica` resource its own dictionary does not define.
- `issue19182.pdf` names `UniCNS-UTF16-H`, one of the predefined `CMap`s this tree refuses.

Both leave the oracle's judged set, so agreeing falls 841 → 839 — a smaller denominator, not a
worse renderer.

**And a text-extraction shortfall nobody had diagnosed was a font nobody had looked up.**
`issue19971.pdf` sat on `TEXT_BELOW_FLOOR` at 83% among "six partial for reasons nobody has
diagnosed further"; it is above the floor now. `issue19182.pdf` leaves that list too, by leaving
the gated population. The list is 44 → 42, and four of the "undiagnosed" six remain.

Tests 894 → 895. The new one was confirmed to fail against the old key.

## The lesson

**The measurement that finds a defect need not be about the defect.** This one began as a
question about `zlib_rs` and 28% of interpretation, and the duplicate inflation it was chasing
is *still there* — two distinct font dictionaries in that file share one `/FontFile2` object,
which keying by the font dictionary does not collapse. The perf item is unfinished; the
correctness bug it walked past is fixed.

**And a cache is a claim that two things are the same.** Every cache in this tree now keys on
object identity — `shading::Cache`, `MaskCache`, the font cache — and the one that did not was
the one that made the claim in the weakest possible currency: a name the file may reuse.
