# ADR 0516 — Eight negatives re-derived, and the population a refusal is actually about

Status: accepted, 2026-08-23. Session the six-hundred-and-eighty-second, a clause round under
`doc/todo/01`'s sixteenth sweep. Adds eight claims in nine blocks to `examples/absence_audit`,
amends the ledger rows of §7.6.5, §7.9.2.2.2, §8.9.5.2, §8.10.3, §11.6.5.2, §12.3.2.2, §12.4.2 and
§12.5.1, corrects two source claims in `crates/pdf-model`, adds one section to `doc/errata-read.md`
and rewrites `doc/todo/01`'s second group. Extends ADRs 0405, 0490, 0493, 0496 and 0502; changes
nothing ADR 0321 or 0399 decided about soft masks.

## 1. What this decides

`doc/habits.md`'s decay shape — **a negative claim decays when the population grows** — applied to
the eight rows `doc/todo/01` named as needing a structural block. `absence_audit --crawl` asks each
of them through the reader that would act on it, over both populations separately (ADR 0490's
control-and-growth rule).

| clause | the claim | curated 1251 | crawl 65 944 |
|---|---|---|---|
| §7.6.5 | no corpus document uses a public-key handler | 0 | **1** |
| §7.9.2.2.2 | no corpus document writes a language escape | 0 | **1**, and it is a *lone* escape |
| §8.9.5.2 | every image writes Table 88's default or its reversal | holds | **false by 7** |
| §8.10.3 | no corpus document writes a `/Group` subtype that is not `/Transparency` | 0 | **0** |
| §11.6.5.2 | no corpus document states a codec-carrying `/SMask` | **6** | **2882** |
| §12.3.2.2 | no corpus link uses the integer first element | **5**, all remote | **599**, 44 of them local |
| §12.4.2 | no corpus document exercises all three of the example's ranges | **1** | **11** of 722 |
| §12.5.1 | no corpus document states a rotated page with a widget | 0 | **15** |

Seven of the eight were false. What the round adds beyond seven counts is three rules, and the
third is the one that changed a conclusion rather than a number.

## 2. A negative can be false and nothing be owed

§7.6.5's single crawled witness — `3006236.pdf`, `/Filter /Adobe.PubSec` — is a file this tree
declines by name: `Document::open` returns `SyntaxError::UnsupportedEncryption` quoting the handler,
which is trap 5's loud refusal rather than a page drawn wrong. The clause still needs an
`EnvelopedData` reader, an RSA decryption and a route to the reader's own private key, and one
document does not change what that costs. The row gains the count and keeps its status.

This is worth stating because the reflex from a false negative is that work follows. It does not
follow from the count; it follows from what the code does when the document arrives, which is a
separate question and the one to ask.

**The instrument is unusual here and that is the point.** Every other block in `absence_audit` asks
a reader about an open document. This one cannot: the document does not open. So the block reads
the *refusal*, which is exactly what §7.6.5's row claims happens — the row's own sentence is that a
non-`Standard` `/Filter` "produces `SyntaxError::UnsupportedEncryption` quoting the name", so
asserting the error is asking the reader. A name census over the bytes would have answered a
different question.

## 3. A negative can be false while the residue it justifies survives one condition narrower

§11.6.5.2 is `partial` for three residues, the first being "a mask behind an image codec, whose
samples have no position until the whole codestream is decoded", and it closed with "no corpus
document states a codec-carrying mask". That sentence is false by 6 curated documents and 2882
crawled ones.

**The count is not the residue.** `soft_mask_entry` consults `eligible_for_the_device_scale` — the
function that asks about the codec — only where `worth_combining` has already said the refinement of
the two grids is too large to build. So a codec-carrying mask on an ordinary pair is combined on the
finer grid and drawn, whatever it is filtered with; the sentence is about the pair that *reaches*
the codec test, and nothing was counting that.

Counted, it is **6 documents of the 65 944 and none of the 1251** — `0450015`, `2514975`, `2637184`,
`4851379`, `5097994`, `7434197` — and what they get is the eager combination the deferred route
exists to avoid, up to `MAX_SAMPLES`, past which `combined_grid` refuses by name. That is a cost
rather than a wrong pixel, and it is a population a later round can open.

This is trap 11's rule pointed at a *census* instead of at a report: a block that fires on the noun
in the sentence rather than on the condition the sentence states measures the wrong thing, and it
measures it three orders of magnitude too large. The block is split in two — the claim's population
and the residue's — because one report line cannot say both.

`image.rs`'s own doc comment carried the same false sentence ("No corpus document states one") and
is corrected with both figures. That is `doc/habits.md`'s "a retired claim is a string" again: the
ledger and the code stated one negative in two places and only one of them was on any sweep's list.

## 4. Probe a positive as well as a zero

`doc/habits.md` and `doc/todo/01` both say to plant a witness before believing a zero, and this round
did — a hand-built file stating all eight constructs, dropped into `doc/corpora-own` for one run and
deleted, and every one of the eight blocks saw its construct. Two of the eight would otherwise have
been believed on a first run, and one of them was `/DCTDecode` behind an `/SMask`, which is the
2882-document population.

**The new half is the other direction.** The first draft of the `/Decode` block reported
`issue10339_reduced.pdf` as a departure from Table 88, on `/Decode [255.0 0.0]`. It is not one: the
image is eight-bit `Indexed`, whose Table 88 default is NOTE 2's `[0 2^n − 1]` — `[0 255]` — so the
stated array is that default's *exact reversal*, which is precisely what §8.9.5.2's row claims every
corpus image writes. A census that compares against one family's default and calls `[1 0]` "the
reversal" retires a claim that holds. The block builds the three defaults an image dictionary can be
measured against without its resources — every full-range space's, `Indexed`'s from
`/BitsPerComponent`, and `Lab`'s from the space array's `/Range` — and accepts each of them and each
one's reversal.

So: **a positive is as capable of being an instrument defect as a zero is**, and the tell is the
same — a hit nobody expected, in a population somebody has already measured.

## 5. Two negatives that survived, and what each is worth

- **§8.10.3** — zero over both populations, with the planted `/Group << /S /Softness >>` seen. So
  §11.6.6's second half ("shall not be subject to any grouping behaviour") has no file behind it at
  all on a population sixty-eight times the one the claim was written over, and
  `a_form_becomes_a_group_only_for_the_transparency_subtype` is the only witness the requirement
  has. That is trap 8 stated as a measurement rather than as a worry.
- **§7.9.2.2.2's sharper half** — one crawled document states U+001B inside a Unicode text string,
  and it is a lone escape rather than either of the two shapes the clause defines. So `text_string`
  leaves it where it stands, which is what `a_lone_escape_does_not_swallow_the_rest_of_the_string`
  already asserts, and the construct whose language would have to be carried through every
  text-string return type in the tree is written by nothing in either population. The row's reason
  for not paying that cost is unchanged and now has 67 195 documents behind it instead of 974.

ADR 0496 §5's rule — a negative can be false and its sharper half survive — is here twice more:
§12.3.2.2's "no corpus link uses it" is false by five curated documents and every one of the five is
the form the clause's own NOTE describes, so the half that matters was true; and the crawl then
falsifies *that* half too, with 44 documents stating an integer first element on a destination
naming no other file, for which `Destination::page_index` answers `None` and the link does nothing.

## 6. What is not decided

- **No behaviour changed.** Every one of the eight rows keeps its status. §11.6.5.2 stays `partial`
  on the same three residues, §12.3.2.2 stays `partial` on the same wait, §12.5.1 stays `partial` on
  the clause's last paragraph.
- **The 44 local integer-first destinations are not acted on.** §12.3.2.2 says the first entry
  "shall be an indirect reference to a page object (except in a remote go-to action …)", and a
  document that writes an integer there has written something the clause excludes. What a reader
  owes such a file is a question for a round with the clause open, not a fallback invented beside a
  count.
- **`§14.8.2.5.3` was moved rather than measured.** `/ReversedChars` is a marked-content tag inside
  a content stream, so a structural block over the object graph would report a false zero for it.
  It belongs with the five claims that need a content-stream census, and `doc/todo/01` says so now.
