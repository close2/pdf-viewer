# ADR 0626 — The codec a boundary waits on, and the two producers it names

Status: accepted, 2026-08-25. Session 732. Carries out ADR 0607's decision, which settled
`doc/todo/34` §2 and which `doc/todo/15`'s road B waits on. Cites no clause: this is `CLAUDE.md`
principle 3's boundary and principle 2's page, and the ledger is untouched.

## What this is, and what it is not

ADR 0607 decided **display lists cross, and the raster payload stays**, chosen per page by size.
It deliberately did not build the codec, calling it "a two-sided encoder plus a fuzz target plus a
deferred-producer question". This is that codec: `viewer_confined::protocol::display_list`,
reachable as `wire::encode_display_list`, `wire::display_list` and `wire::crossing`.

**It is not yet on the frame path**, and that is a scope decision rather than an omission —
section 6 says exactly what remains and why it is a round of its own.

## 1. The direction of trust is the whole design

The encoder runs in the confined process, which holds a hostile document. The decoder runs in the
**host**, which is not confined. So this is a parser in the privileged process over bytes an
attacker who had already subverted the renderer would choose — the direction the seven-hundred-and-
nineteenth session found completely unguarded, when a subverted worker could have claimed a 2 GiB
frame and the host would have asked its allocator for it (ADR 0597). Four rules follow, and each is
a place where a length could otherwise become an allocation or an index:

- **The tables precede the body.** Paths, image samples, shading kinds and shadings are written
  first, then the clips, the soft masks and the commands that index into them. So every identifier
  the decoder reads is checked against a length **it established by reading a table**, never
  against a length the message asserts. That layout is the single largest security property here,
  and it costs the encoder one deferred buffer (`Writer::append`) because the interning that fills
  the tables happens while the commands are written.
- **A count is bounded per table, at the smallest record that table admits.** `Reader::list` can
  only assume one byte an element; a clip cannot be under 34 bytes and a ramp stop cannot be under
  20, so multiplying refuses a claim of 2^28 clips in a nine-byte message before anything is
  reserved. The *reservation* is separately bounded by `RESERVE`, which is 719's own rule.
- **Nesting is bounded by `pdf_render::MAX_GROUP_DEPTH`.** Not an invented number: it is the depth
  every backend in this tree already refuses past, so a list this decoder accepts is a list a
  backend can composite. A group, a `Shaped` element and a group's §11.7.2 black companion all
  count toward it.
- **A structural invariant a backend depends on is checked here rather than assumed there.** An
  `Image` whose stated dimensions its samples do not fill is refused by name — the same check
  `fuzz/fuzz_targets/confined_wire.rs` already asserts of a `Raster`, for the same reason: a
  rasteriser that trusted the dimensions would read past the end of the buffer. So is a ramp with
  no stops, which `Ramp`'s own documentation says never happens and every reader of one depends on.

## 2. Sharing, which is a correctness-shaped performance requirement

ADR 0607 measured what flattening costs: 0.37 → 0.91 of the raster in aggregate, 30× worse at the
extreme. So four tables are interned by `Arc` *identity* — `Arc<Path>`, the `Arc<[u8]>` of an
image's samples, `Arc<ShadingKind>` and `Arc<Shading>` — and every later occurrence is four bytes.

**`Arc<ShadingKind>` is a table of its own, which ADR 0607's accounting did not have.** The two are
separately shared: `pdf_model::shading::Cache` hands one kind to many shadings differing only in
their transform, which is `bug1721218_reduced.pdf`'s 3576 paints from three function objects. It
costs one more table and it is the difference between 3576 mesh geometries and three.

Identity rather than structural equality, deliberately: this preserves the sharing the interpreter
produced and does not go looking for sharing it did not. A deduplicating encoder would be a second
`clip_index` in a different crate, paid on every page to help pages that do not exist.

## 3. Two decisions about the clip table that a smaller design would have got wrong

**The whole table crosses, not only the reachable part.** `examples/list_against_raster` priced
only the clips something references, because that is what a *cost* estimate should count. A
`ClipId` is an index, so dropping an unreferenced entry renumbers every identifier after it. The
table is written whole.

**And the decoder checks that rebuilding it reproduces the message's own numbering.**
`DisplayList::add_clip` hands back the identifier of a region *already in the table* — which is why
page 6 of ISO 32000-2 builds one clip mask instead of 303 (ADR 0132) — so a message stating one
region twice would silently renumber everything after it, and every command naming one would then
clip by the wrong region. On the host's side of the boundary. The decoder compares what `add_clip`
returned against the index the message is at, and refuses. A clip's parent is separately required
to sit strictly *before* it, which is what `add_clip` guarantees of any table it built and what
keeps `DisplayList::clip_bounds` from walking a cycle.

Both are `ProtocolError::Unbuildable`, a new variant whose whole subject is a message that is well
formed and describes a value the type does not admit. It is not a truncation and not an unknown
discriminant, and conflating the three would have made the refusal unreadable.

## 4. The two deferred producers, refused by name

`ImageSource::AtDeviceScale` and `ShadingKind::Sampled` carry `Arc<dyn ImageAtDeviceScale>` and
`Arc<dyn ColoursAtDeviceScale>` — producers the backend invokes once it knows how many device
pixels the mark covers (ADR 0210). `encode` returns `Uncodable::DeferredImage` or
`Uncodable::DeferredColours`, naming the variant, and `crossing` turns that into ADR 0607's raster
arm rather than into a failure. That is trap 5 — unsupported input stays loud — and it is what
makes this a documented boundary rather than a page that quietly went blank.

`Uncodable` has three other variants and each is a refusal rather than a bug: `TooDeep`, `TooMany`,
and `Unknown`, which is the wildcard arm four of `pdf-render`'s enumerations force the compiler to
require. A variant added to `Command`, `Paint`, `ImageSource` or `ShadingKind` therefore refuses by
name instead of being drawn as something else; the *closed* enumerations — `BlendMode`,
`FillRule`, `LineCap`, `LineJoin`, `PathCommand`, `SoftMaskKind`, `Corners` — are written out both
ways, so an addition to one of those is a build failure in this file, which is stronger.

## 5. What it measures, re-derived rather than quoted

ADR 0607's figures came from walking a list and summing what an encoder **must** write. This is the
encoder, so `examples/list_over_the_wire` runs it and the column is a measurement. 958 first pages
of `doc/pdf.js`, byte counts and therefore load-immune, every page decoded and compared against the
list that was encoded before its number was printed:

| | ADR 0607 predicted | this codec writes |
|---|---|---|
| median list/raster at 1.0 | 0.034 | **0.0368** |
| p90 at 1.0 | 1.000 | **1.0002** |
| p99 at 1.0 | 17.3 | **17.36** |
| worst at 1.0 | 1101× | **1101.09×** |
| median at 1.333 | 0.019 | **0.0207** |
| p90 at 1.333 | 0.562 | **0.5619** |
| worst at 1.333 | 659× | **658.77×** |
| pages crossing as pixels at 1.333 | 41 of 957 | **46 of 957**, 3 of them deferred |

**The prediction held to within about a tenth**, and where it is off it is off in the direction a
real format is: the aggregate is 0.4225 against a predicted 0.37 at 72 dpi, which is the tables,
the tags and the length prefixes. Nothing in ADR 0607's decision moves.

The four pages carrying a producer are named rather than counted: `function_based_shading.pdf` and
`function_based_shading_cmyk.pdf` (§8.7.4.5.2's type 1 shading), `issue16263.pdf` and
`issue19517.pdf` (§11.6.5.2's soft-mask image on a grid of its own — the 2×2 image with a
34862×4332 mask). All four cross as pixels, which is what the raster arm is for.

**And 954 of 954 encodable pages round-tripped to an equal list**, `DisplayList`'s own `PartialEq`
comparing both tables, every command, the clip index and the §11.4.7 pair. `tests/confined.rs`
additionally rasterises a decoded list with `render-cpu` and compares the samples against the page
that was sent, because two lists comparing equal is an assertion about a data structure and what a
reader sees is pixels (trap 1).

## 6. What remains, precisely

**The codec is not wired into `Answer::Frame`.** Wiring it is a round of its own and here is why,
rather than as a promise:

- `Reply::Frame(Vec<Framed>)` carries a `Raster` per page and `viewer-ui` consumes it. Turning that
  into a payload choice breaks every consumer, which is what `#[non_exhaustive]`'s absence is for
  and which is a change worth making deliberately.
- **The host would have to rasterise**, so `viewer-confined` would take a rasteriser as a
  dependency — and *which* one is a decision with an argument: ADR 0607 says "`render-quorra` is
  that translator and `viewer-ui` is already on it", which means the host that draws is
  `viewer-ui` rather than `viewer-confined`, and the crate boundary follows from that rather than
  from convenience.
- The frame protocol's `MAGIC` is `PDFVCF03` and gates build compatibility. Wiring this in changes
  the format incompatibly and therefore bumps it. It is **not** bumped here: nothing this round
  added is carried in a frame, so bumping now would refuse a worker that speaks the same protocol.

**The two producers stay deferred**, and ADR 0607's argument for that is unchanged and is repeated
in the module. What would change it is a page population where the raster arm is not adequate; the
instrument is `examples/list_over_the_wire`, and today's answer is three pages of 957.

## 7. What the fuzz target found on its first run, which was not a decoder defect

`fuzz/fuzz_targets/display_list.rs` failed at **750 executions**, reporting `one message decoded
two ways`. It was not: the input stated NaN, `DisplayList`'s `PartialEq` is ultimately `f32`'s,
and `f32`'s is **not reflexive** — so a perfectly decoded list was unequal to itself and an
`assert_eq!` over two identical decodes failed.

Two answers were available and the choice matters, which is why it is here rather than in a
comment.

**The decoder does not refuse a non-finite number.** The premise of this whole boundary is that
the confined path draws the page the in-process path draws — `tests/confined.rs` asserts it to the
byte — so a value the *interpreter* can produce may not be refused at the transport. The three
places that already ask the question are `Transform::invert`, `thinnest_line` and
`Grid::for_placement`, all in `pdf-render`, where a device decision belongs; a fourth answer in a
codec would be a fourth place the two paths could differ. The corpus produced none, incidentally:
all 954 encodable pages compared *equal* on the round trip, so nothing this interpreter writes
today is affected either way.

**So the assertion changed, not the codec.** The target compares the bytes of a re-encoding
instead, which says the same thing about determinism and about the round trip and says it for
every input rather than for the finite ones — and it is the stronger statement besides, because it
asserts the encoding is a **canonical form**: a field dropped, reordered or widened on one side of
the codec fails it. `a_non_finite_number_crosses_and_the_encoding_is_what_compares` is the
permanent regression test, which is `CLAUDE.md` principle 3's rule for a crasher.

With that assertion, the target is **clean at 4 175 795 runs in 901 s**, seeded from 749 real
pages, reaching 2709 edges and a corpus of 842 inputs. Nothing else came out of it.

## 8. One addition to `pdf-render`, named

`ClipId::new(u32)`, which did not exist while `SoftMaskId::new` did. A decoder rebuilding a table
it was handed has to name entries in it. It is documented as `SoftMaskId::new` is — only
`add_clip` should *mint* one — and the two callers it exists for are a backend's tests and a
decoder reading an identifier back.
