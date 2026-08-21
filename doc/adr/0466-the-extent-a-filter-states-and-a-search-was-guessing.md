# ADR 0466 — The extent a filter states, and the search that was guessing at it

Status: accepted, 2026-08-21. Session 633. Takes `doc/todo/03` §21's named successor —
`7926872.pdf`, left with a diagnosis and no fix — and amends §8.9.7's and §7.3.8.2's ledger rows.
Corrects a claim about the standard that `crates/pdf-model/src/inline_image.rs` had carried since
the eleventh session.

## The defect, in one sentence

An inline image whose data is *filtered* and which states no `/L` had the end of its data
**searched for** rather than **derived**, so a byte pair inside 2.9 MB of Flate that happened to
read as a white-space-delimited `EI` ended the image 24 822 bytes in.

`7926872.pdf` page one is one command: a 1200×1790 `/DeviceRGB` photograph, `/W 1200 /H 1790 /CS
/RGB /BPC 8 /F /FlateDecode` and no `/L`. Before this round:

```
1 commands, unsupported [Image { name: "<inline>: its samples stop at 477217 bytes where
1200x1790 at 8 bits and 3 component(s) needs 6444000 (§7.3.8.2 infers the extent from the
dictionary); what the stream carries is drawn and the rest of the grid is left unpainted" },
Operator { operator: "\u{1}$�|�-z\u{351}�|X inside an array, which §7.3.6 admits only objects
into" }, … ]
```

477 217 samples of 6 444 000 — 7% of the picture — and the remaining 1.4 MB of the photograph
handed to the lexer, which is where the array-operand reports come from. Ink **2.915** of 255
against `pdftoppm` 45.233, `mutool` 45.020 and `gs` 44.647.

## What the standard says, and it says it twice

§8.9.7 does not answer the question itself. It sends the reader one clause over:

> The bytes between the ID operator and a white-space token, but before the EI operator shall be
> treated the same as a stream object's data ( see 7.3.8, "Stream objects"), even though they do
> not follow the standard stream syntax.

And §7.3.8.2 answers it:

> In addition, most filters are defined so that the data shall be self-limiting; that is, they use
> an encoding scheme in which an explicit end-of-data (EOD) marker delimits the extent of the
> data.

So a filtered extent is **derivable**, and the forward search was never the only answer available
for it. This is `doc/traps/parsers-and-streams.md`'s trap 5 in the form ADR 0356 gave it — *ask
first whether the standard states the thing's extent* — and it is the fourth clause family that
rule has now moved.

Errata Collection 3 sharpens the answer rather than changing it. Issue #319 adds a NOTE at
§7.3.8.2 saying that the encoded data "encompasses all enveloping markers of the encoding, e.g.
end-of-data markers, if the encoding scheme uses them", which is why the extent this round derives
is the decoder's **consumed input** and not the offset of the last byte it needed: for a zlib
stream that includes the Adler-32 trailer, and for `LZWDecode` the byte holding the last bit of
§7.4.4.2's EOD code.

## The population, measured before the change

Trap 11's rule, and it took `examples/token_window_census` — which already classified every inline
image by which of §8.9.7's answers decides its extent — plus one comparison: ask the first filter
of the chain where its own marker stands, and compare that with where the scan stopped.

Over the **65 967** crawled documents that open (`corpus-cache`, 926 781 pages) and the **1 257**
curated ones under `doc/`:

| | crawl | curated |
|---|---|---|
| inline images read | 3 977 492 | 6 410 |
| filtered, no `/L` — the population at issue | 2 672 062 | 2 975 |
| of those, a marker this tree can locate | 1 366 627 | 2 518 |
| of those, agreeing with the search | 1 366 610 | 2 518 |
| **of those, ending early** | **17, in 5 documents** | **0** |
| encoded bytes taken for content operators | 13.45 MiB | 0 B |

The first filter of the 2 672 062, by name: `FlateDecode` 1 367 073, `CCITTFaxDecode` 1 272 430,
`ASCII85Decode` 23 018, `ASCIIHexDecode` 4 104, `DCTDecode` 3 778, `RunLengthDecode` 1 655, and 4
whose `/F` is not a name.

**Two things in that table are worth as much as the 17.** The curated corpora — every population
the corpus, oracle, quorra and text gates walk — carry **not one** instance, so this defect was
invisible to every ratchet in the tree, which is `CLAUDE.md`'s two denominators exactly. And the
`CCITTFaxDecode` row is larger than the `FlateDecode` one: half the population still has its end
searched for, because §7.4.6's filter has no resumable decoder here. That is named in the ledger
and in `doc/todo/03` rather than left as an assumption.

**The same census re-run over the whole crawl after the change reports 0 early of 1 366 702
locatable**, and two more images past a mebibyte — the two photographs. Its totals move by **75 of
3 977 492**, which is not noise to be waved at: where an inline image ends decides what the rest of
the content stream lexes as, so an image that used to end early gave the walk a different stream
afterwards. The two runs do not have quite the same denominator, and a defect of exactly this shape
is the reason.

**The independence caveat is stated rather than assumed** (trap 8): the comparison needs both
answers, so it is not independent of the code under test in the way a census with an outside
instrument would be. What makes it worth something is the direction: the *predicate* is the
existing scan's answer, the *reference* is a decoder that predates this round, and the finding was
then confirmed by rendering the two documents it names against three other implementations.

## The change

Three pieces, each in the crate that owns the clause.

**`pdf_syntax::filter::encoded_extent`** drives a resumable decoder over bytes it throws away a
window at a time and reports how many of its input's bytes the filter's marker delimits. It
answers three ways — `Ends(n)`, `Short`, `Unknown` — and the middle one is the whole reason it is
an enum: a caller reading through a window has to tell *these bytes carry no marker* from *these
bytes ran out before one*, which are a statement about the file and a statement about the buffer.
Its output bound is `Limits::max_stream_len`, which everywhere else in this crate bounds an
allocation and here bounds time instead, because nothing is kept.

It shares its decoders with `Pump` rather than owning a second copy: `Engine::new`, `Engine::pump`
and `Engine::consumed` are factored out of `Pump` and both drive them. `Pump` already counted its
consumed input on both engines and exposed it to nobody, which was the whole of what stood between
the guess and the answer.

**`pdf_syntax::Document::filtered_extent`** picks the filter. It is the **first** of the chain,
because Table 5 orders a chain "in the order in which they are to be applied", so the bytes in the
file are the first stage's input — `[/A85 /LZW]` is a question for `ASCII85Decode`. A predictor is
not consulted and does not need to be: §7.4.4.4's predictor runs over a stage's *output* and moves
no byte of its input, which is the one respect in which this question is easier than
`stream_source`'s.

**`pdf_model::inline_image::data_extent`** gains it as answer 3 of four, between §8.9.3's
arithmetic and the search, and it is checked against the `EI` it predicts exactly as the other two
are. Through a window, a filter that runs out of input answers `Extent::PastTheBuffer` — a request
for more bytes — rather than falling through to the search, which is ADR 0454's shape kept rather
than re-derived: without it the derivation would be dropped for exactly the images it was written
for, since an image large enough to matter is an image larger than the window.

**And `terminator_at` became three-way for the same reason.** The check for the `EI` a derived end
predicts used to answer yes or no, with the caller inferring "the buffer is too short" from `end >
content.len()` — which is right when the end is past the buffer and wrong when the end is inside
it and the white space or the keyword straddles the edge. All three derived answers now get the
same `Terminator::PastTheBuffer` and the caller applies `complete` to it in one place.

## What it is pinned by

Trap 8's rule is that a corpus finds what documents contain, so the pair is hand-built and the
mechanism is what makes it honest: **`flate_stored` writes RFC 1951's *stored* block**, which holds
its payload literally, so the encoded data really does carry ` EI ` and a test can say so with an
assertion rather than a hope.

- `filtered_data_ends_at_the_filters_own_end_of_data` — twelve samples whose middle four spell
  ` EI `, under `FlateDecode` with no `/L`. The data must be the whole stream.
- `filtered_data_without_an_ei_inside_it_ends_where_it_always_did` — the twin, same construction,
  no `EI` anywhere in the encoded bytes, where both answers agree. Without it the pair would say
  only that *something* changed, not that the marker is what decides.
- `a_window_that_cuts_the_filters_marker_asks_for_more_bytes` — the same image through a buffer cut
  two bytes short of the end of its data, and well past the ` EI ` inside it, which must be
  `InlineImageError::Truncated`.

Trap 13, run rather than assumed: with `crates/pdf-model/src/inline_image.rs` reverted and nothing
else, the first and third **fail** and the twin passes.

## What it is worth

| | before | after | `pdftoppm` / `mutool` / `gs` |
|---|---|---|---|
| `7926872.pdf` p1 ink | 2.915 | **44.516** | 45.233 / 45.020 / 44.647 |
| `4605499.pdf` p1 ink | 8.848 | **71.775** | 72.682 / 72.409 / 72.062 |

Both report nothing now where both reported a short image and a stream of array-operand
complaints. `4605499.pdf` is a **second** document of the five and was not in §21's head at all —
its archive was in none of the ten that chunk took — and at −63.2 it is deeper than the row this
round was sent after.

The interpretation of `7926872.pdf` page one also fell from 785 ms to 101 ms in the same run and on
the same machine, which is not a performance result and is not offered as one: it is 1.4 MB of
photograph no longer being tokenised.

Both are rows in `doc/checks/fixed-documents.toml`, which is `doc/todo/03` §20's rule and the only
gate that will see them at a merge.

## An erratum found by running `emit` before writing

`doc/errata-read.md`'s rule — *a round implementing a clause runs `spec-errata emit` on that
document before it writes* — was followed, and it paid immediately on a clause this round was not
changing.

`expand_key`'s doc comment said, of a file that writes both an abbreviated key and its full name:

> The standard states no rule for that, here or in §7.3.7, and this is therefore a decision rather
> than a reading.

**Errata Collection 3 states the rule.** §8.9.7, Issue #3, `/State` `Review` `Completed`, a caret
inserting "[i]n the situation where both an abbreviated key name and the corresponding full key
name from Table 91 are present, the abbreviated key name shall take precedence" — which is exactly
what this tree does, arrived at from `issue14256.pdf`'s bytes and recorded as a *choice* for six
hundred sessions. The code did not move; the comment did. `CLAUDE.md` principle 5's "'the
specification defines nothing here' is itself a claim about the specification, and it decays" has
now decayed a third documented time, and `emit` is the instrument that catches this shape:
`check` compares quotations the tree has written, and no quotation of an *inserted* sentence can
exist before somebody writes it.

## What is not done, and why it is named rather than assumed

**Five of §7.4's filters state an end-of-data this tree does not ask for**, and the population
above says what that is worth: `CCITTFaxDecode` decides 1 272 430 of the 2 672 062 filtered inline
images in the crawl, and `ASCII85Decode`, `ASCIIHexDecode`, `RunLengthDecode` and `DCTDecode`
another 32 555 between them. The reason is `Pumping`'s and not the clause's — only `FlateDecode`
and `LZWDecode` have a resumable decoder here — and three of the five would need no decoder at
all, their markers being `>`, `~>` and a length byte of 128. `ASCIIHexDecode` is provably safe
without one: its alphabet is the hexadecimal digits, white space and `>`, and `I` is not among
them, so no `EI` can stand inside its data. The other four are exposure this round measured the
size of and did not close.

**And 446 of the crawl's `FlateDecode` inline images have no locatable marker at all** — their
data is corrupt before one — where the search remains the answer and is the right one. That is
what `EncodedExtent::Unknown` is for, and it is why the search is still in the module rather than
deleted from it.

## The cost, stated

`filtered_extent` decodes the data once to find where it ends, and whoever wanted the samples then
decodes it again. That is deliberate: the alternative is holding a decoded buffer of unbounded
size across a scan whose whole purpose is to avoid one (ADR 0365, `doc/todo/14`). It replaces a
linear walk over the same bytes looking for `EI`, so the population it runs on is one where a pass
over those bytes was already being paid for; what it adds is the decompression, on filtered inline
images that state no `/L` and nothing else in the tree. The curated corpora's counts and the gate
sequence are unchanged by it.
