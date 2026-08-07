# ADR 0223 — A panel is eleven answers, and none of them is a number

Status: accepted, session 386.

## What was decided

**All twenty-five of `viewer-core`'s questions now cross the confinement.** The eleven that did
not — `Outline`, `Layers`, `Attachments`, `Collection`, `Articles`, `Thumbnail`, `Properties`,
`Opening`, `Preferences`, `Popups`, `AccessibilityTree` — are the eleven that answer with a
`pdf-model` type rather than with a count, a rectangle or a flag, and ADR 0218 left them because
encoding a document model is a different job from encoding a message. `viewer_confined::protocol::panels`
is that job: one encoding per type, beside the twenty-eight that were already there, with a round
trip apiece.

A host on this boundary had no panels at all. It has six now, and the demonstration is
`examples/confined_panels` — an outline printed as a tree, a layer list, an attachment list, a
thumbnail's dimensions, an XMP packet's `dc:title` and a page's structure, out of a process that
cannot open a file.

## The property this had to keep, and what it cost

ADR 0218's boundary rests on one thing: **the compiler names the message nobody handled.** Every
`match` in `protocol.rs` is exhaustive over a `viewer-core` enum, and nothing in that crate is
`#[non_exhaustive]`, so a variant added there fails to compile here.

A *struct* has no arms, and that is the whole difficulty of this round. `pdf_model::outline::Item`
has eight fields; an encoder written as `writer.str(&item.title)` would keep compiling on the day a
ninth arrives, and what a person would see is a panel showing less on the confined path than off
it. No gate in this tree looks at a panel.

So **every encoder here opens with a `let` that names every field**:

```rust
let Item { id, title, destination, open, italic, bold, colour, children } = item;
```

A pattern with no `..` is exhaustive in exactly the way an arm list is. That is one line per type
and it is the reason the module reads the way it does, rather than the shorter way.

**One field is deliberately not carried, and it is named in the type.** `pdf_model::attachment::Attachment`
ends in `stream: Arc<Stream>`; `viewer_confined::Attachment` has no such field, and the encoder's
pattern says `stream: _` with the reason beside it. §7.11.4's bytes already had a channel —
`Command::Extract` names one file and `Event::Extracted` brings it back — and a list of five
attachments that pulled five payloads across a pipe to draw five rows would be paying a document's
whole weight for a list. The asymmetry of the pair (`Attachment` in, `crate::Attachment` out) is
where the omission is stated; a comment could have been deleted.

## Three things are refused, and each names what it refused

The transport had two refusals and now has five, all of the same shape: an `Uncarried` carrying the
variant's name and a sentence, delivered as a *refusal frame* so the worker stays alive.

- `Command::RenderReady` and `Event::NeedsRender`, ADR 0218's two, because the confined process
  answers them itself.
- **A raster in a second pixel layout.** `RasterFormat` is `#[non_exhaustive]`; ADR 0218's.
- **A collection subitem's `/D` that is not Table 47's three kinds.** `pdf_model::collection::item`
  has already unwrapped Table 46's dictionary form into `/D` and `/P`, so what reaches the encoder
  is the subitem's own data — "text string, date, or number" — and an array, a dictionary, a stream
  or a reference is a file that wrote something the clause does not describe. Refusing the answer
  by name beats flattening it into a string a host could not tell from one the file wrote.
- **An `XmpError` variant this build cannot name.** `XmpError` is the one `#[non_exhaustive]` type
  in this vocabulary, so its `match` cannot be exhaustive from another crate, and the wildcard arm
  refuses rather than inventing an error it can spell.

The `TooMuch` variant is worth a sentence of its own: its payload is a `&'static str` naming one of
`pdf_model::xmp`'s four budgets, and a `&'static str` cannot be made from bytes that arrived at run
time. The four are matched back to their own literals on decode and a fifth is a `ProtocolError`,
which is stricter than it needs to be today and cannot leak a string.

## What the untrusted side made this round change, and one of them was already wrong

The confined side is the untrusted side: a subverted worker chooses the bytes the *host* decodes.
Three things follow, and the first is a defect that predates this round.

**A length was checked and a reservation was not.** Every list decoder refused a count larger than
the bytes left — correctly, since an element costs at least one byte — and then called
`Vec::with_capacity(count)`. `MAX_MESSAGE` is two gibibytes, so a message of nine bytes claiming
2^31 strings had the host ask its allocator for `2^31 × 24` bytes and abort. The check bounded the
*loop* and not the *allocation*, which is a distinction a fuzzer at small input sizes never reaches.
`Reader::list` is now the single place a list is read: it keeps the count check, reserves at most
`RESERVE` = 256 elements and grows into the rest. Four existing decoders were moved onto it.

**Four of the eleven answers are trees**, and a decoder that followed one as deep as it was told to
would exhaust the host's stack from a few hundred bytes. `ProtocolError::TooDeep` at 64 is a bound
on a *message*: `pdf_model::outline` stops at 32 and §8.11.4.3's `/Order` and §12.3.5.2's folders
are bounded the same way, so reaching it means the bytes did not come from those readers.

**§14.7's answer is a flat list with parent *indices*.** A host walks them upwards; a node naming
itself, or naming one that has not been read, is a loop rather than a tree. The confined side
produces the answer parent-first by construction, so the check is one comparison per node and it is
in the decoder rather than in the host's assumptions.

## The fuzz target this round owed

`viewer-confined`'s decoders were guarded by a deterministic truncation-and-byte-flip test standing
in for a fuzz target, which was proportionate to twenty-eight encodings of scalars and stopped being
so. `fuzz/fuzz_targets/confined_wire.rs` is the real one, over all four decoders, and it needed
`viewer_confined::wire` — the reading half of the transport, public, because a fuzz target lives
outside the crate and cannot reach a private module. The worker's own two decoders are in it for the
mirror-image reason: `pdf-view-worker` is a *program*, so its standard input is whatever was piped
into it.

**Its corpus is seeded by a second implementation of the format.** A twenty-line Python script
speaks the frame layer by hand, spawns the release `pdf-view-worker`, opens five documents and asks
all twenty-five questions — 83 payloads the worker actually wrote, including an outline of 988
items, a decoded thumbnail and a tagged page's structure. That the hand-written encoder and the Rust
one agree on the first try is itself a check.

**It found something on its first run, and the defect was the target's.** A `PageGeometry` whose
page height was all ones decodes to `NaN`, and `NaN != NaN` — so "decoding is a function of the
bytes", asserted with `PartialEq`, fails on a decoder that is perfectly deterministic. The format
carries geometry as `f32` *bits* on purpose (a coordinate through a decimal spelling would be a
different number on the other side), so `NaN` is a legitimate message and the assertion is over
`Debug`, which is total. **44 723 045 runs, clean, under a 1 GiB address-space limit.**

## What it costs, measured

`protocol`'s `what_each_panel_costs_to_cross`, release, this machine, encode and decode of the
answer each document actually gives:

| answer | `PDF20_AN002-AF.pdf` | `PDF-Declarations.pdf` | `ISO_32000-2_sponsored_EC3.pdf` |
|---|---|---|---|
| outline | 3 098 B | 1 976 B | **88 233 B**, 0.043 ms encode / 0.076 ms decode |
| structure (§14.7, one page) | 9 B | 9 B | 13 932 B, 0.005 / 0.008 ms |
| thumbnail | 31 100 B, 0.007 / 0.001 ms | 31 100 B | — |
| properties (Table 349 + XMP) | 1 186 B | 1 620 B | 2 063 B |
| attachments | 9 B | 422 B | 9 B |
| layers | 9 B | 9 B | 80 B |
| everything else | 1–26 B | 1–26 B | 1–27 B |

**The largest answer in the tree is an outline, not a raster**: ISO 32000-2's 988 items are 88 KB
and cost a tenth of a millisecond to cross. A thumbnail is 31 KB because it is a decoded raster and
it is 74×105 — the naive encoding is the right one, and the two candidates for expensive turned out
to be the two that are not. For scale, ADR 0218 measured a *page* at 4.1 MB and 3.4–4.8 ms.

End to end through the pipe, `examples/confined_panels` on `PDF20_AN002-AF.pdf`: outline 0.022 ms,
thumbnail 0.193 ms, properties 0.049 ms, the other eight between 0.006 and 0.025 ms.

**What the eleven add to a confined launch is nothing, and that is a claim with a mechanism behind
it rather than a measurement that came out small.** They are questions, asked when a panel is drawn;
the worker's launch path is `confine()` and a greeting, and neither reads a byte of this module.
Worker start and confinement measured 1.172, 0.957 and 1.056 ms this session against ADR 0218's
1.09, 1.10 and 1.14 ms — the same number.

## The sweep, and the one gap this round closed one layer down

`doc/todo/01`'s fifth sweep, run over `pdf-model` against all four host crates including
`viewer-confined`: **231 `pub fn`s, 84 named by no host** (198 and 71 at its last run). The
populations are the known three, and one hit is this round's own making:

**`Attachment::checksum_matches` was unreachable from a confined host** the moment the stream stopped
crossing. Table 45's rule is about a checksum and the *decoded bytes* together, and this boundary
puts them in two different messages — so a host would have had to reimplement MD5 against a clause,
or not ask. `pdf_model::attachment::checksum_matches` is now a free function; the method calls it,
`viewer_confined::Attachment` calls it, and `tests/confined.rs` asks it end to end on
`PDF-Declarations.pdf`'s two embedded files: list, extract, compare `/Size` against what was decoded.

**One hit is left open and is written into the ledger rather than fixed.** `Collection::initial_document`
answers §12.3.5.1's `/D` fallbacks — the container, a named file, the first file, or "an empty
preview window" — and **no host can call it**, because it needs the `&Document` that only
`viewer-core` holds. That is not a transport gap: `viewer_ui::chrome` cannot ask it either, so the
confined boundary reproduces the in-process one faithfully. Closing it is a field on
`Answer::Collection` and a consumer for it, and it is `doc/todo/34`'s.

## What was considered and rejected

- **Carrying the attachment stream.** It would make the pair symmetrical and make a list cost the
  documents in it. `Command::Extract` exists, and one file at a time is what a person clicking
  *save* asks for.
- **Flattening a collection value to a string.** Table 47 gives three types and says the type
  "shall match the data type identified by the corresponding collection field dictionary"; a string
  standing in for an array would be indistinguishable from a string the file wrote, which is the
  silent-loss failure this round is against.
- **Encoding `pdf_syntax::Object` in general**, so that a collection value could be anything. It
  would drag a stream's bytes and a dictionary's recursion into a panel's answer for a shape no
  clause describes, and the refusal is shorter and truer.
- **Boxing nothing.** `Reply` is as large as its largest variant, and a collection is Tables 153 to
  160 together; `Reply::Count(usize)` would have cost what a collection costs. `Collection`,
  `Information` and `ViewerPreferences` are boxed and the rest are not.
- **A `wire` module with encoders too.** The fuzz target needs the *readers*; publishing the writers
  would be public surface with no caller, which is the thing this project's fifth sweep exists to
  find.
