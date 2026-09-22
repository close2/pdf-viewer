# ADR 1177 — §7.4.8's `/ColorTransform`: a departure whose witness writes the name somewhere else

Status: accepted, 2026-09-22.

Amends ADR 0036, which decided the departure, and ADR 1165, which re-derived it and found its
premise dated rather than wrong. Both stand as records; what this one changes is the row.

## What ADR 0036 decided, and on what

Table 13's `/ColorTransform` is not read by this tree: the codestream decides a `DCTDecode`
image's colour transform, in both directions. ADR 0036 recorded that as a deliberate departure
on one measurement — "[f]our corpus documents write the entry and all four write `0`", of which
one, `issue12841_reduced.pdf`, was "three-component with neither marker, which is precisely the
case where the clause says the dictionary decides", and obeying it would have left a photograph
untransformed where every reference renderer transforms it.

ADR 1165 re-read the argument against the tree and found no expired capability, no property of
the output device and no writer in it. What it did find was a **count**, taken over the 974
curated documents before the crawl existed, and it left the census owed.

## The census, and the two things it found

`crates/pdf-model/examples/colour_transform_census.rs` asks every stream whose `/Filter` chain
ends in `DCTDecode`, and every §8.9.7 inline image, which of Table 13's cases it is in — reading
the component count, the component identifiers and the Adobe APP14 transform byte off ISO/IEC
10918-1's own marker segments, and taking the cases from the clause rather than from any
renderer.

```sh
cargo build --profile gates -p pdf-model --example colour_transform_census
RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh --data 12 --tree 12 -- \
    <target-dir>/gates/examples/colour_transform_census @crawl.txt
```

Over the 65 944 crawled documents, 159 s: **792 516 `DCTDecode` images**, 456 728 carrying an
APP14 marker, 236 739 in the clause's default case, 98 856 with one or two components (where the
option "shall be ignored"), 35 whose markers do not read, and **158 images over 17 documents in
the case the entry decides**. Of those 158, **none would draw differently if the entry were
read**: 83 are four-component images writing `0` and 75 three-component images writing `1`, which
in both counts is the value the codestream already gives. Ten of the seventeen documents were
opened and read by hand against the clause, and ten of ten are the rule rather than the tree —
the name is inside `/DecodeParms` in every one, including the `[null << /ColorTransform 1 >>]`
array form beside a `[/FlateDecode /DCTDecode]` chain, and only one of the ten carries an APP14
segment anywhere in the file, belonging to a different image.

Over the 974 the deciding case is **empty**, and that is the second finding.

**The zero is calibrated, because a clean sweep is a sentence about the instrument** (trap 13).
Two one-byte plants, each into a real crawled document and each leaving every offset in the file
where it was: `3375857.pdf`'s `/DecodeParms << /ColorTransform 1 >>` becomes `0` on a
three-component frame with no APP14, and `7926951.pdf`'s `<</ColorTransform 0>>` becomes `1` on a
four-component one. The census names each planted image as one that would draw differently and
leaves the 82 untouched siblings of the second alone, so the zero over the real population is
about the population.

## The premise was not dated; it was mislocated

`issue12841_reduced.pdf` writes `/ColorTransform 0` as a **direct key of the image dictionary**,
between `/Length` and `/ColorSpace`, with no `/DecodeParms` in the object at all. Both clauses
that mention the parameter put it in one place. §7.4.1: "These optional parameters shall be
specified by the DecodeParms entry in the stream's dictionary". §7.4.8: "the parameter need not
be present in the encoded data but shall be specified in the filter parameter dictionary".

So implementing Table 13's sentence would not touch that file: it states no Table 13 entry, the
clause's default governs it, and the default for a three-component frame with no marker is 1 —
which is what this tree draws today. **The sentence "implementing the clause would break the only
file that exercises it" was true of a grep and not of the clause.** The same is true of the other
three documents ADR 0036 counted; over the whole crawl 5848 images write the name in that place,
3935 of them where honouring it there would change a pixel.

Honouring a name §7.4.1 puts nowhere would be a *new* departure — toward a producer's evident
intent, on `CLAUDE.md`'s robustness question — and it is not taken, because the evidence for it
points the other way. `issue12841_reduced.pdf` is the best-studied member of that population, and
ADR 0036 established that it is right *transformed*: the entry it writes says `0`, and obeying the
producer there would draw a photograph nobody could read. So a producer who writes the name
outside the filter's parameter dictionary is a producer who has not written this parameter, and
the clause's default is a better answer than the entry is. The 3935 are a population somebody can
re-open with a picture; they are not a debt.

## The row is `partial`, not `departed`

`departed` is for a requirement "decided against with its cost recorded". The cost is now
recorded and it is **no pixel on either population**, and the argument that bought the decision
was about a different fact. A departure that costs nothing and buys nothing is not a decision; it
is debt, and the honest status for debt inside an otherwise implemented clause is `partial`.

Two of the clause's three cases are executed — the APP14 case, because this tree follows the
codestream, and the default case, because `zune-jpeg`'s inference for a frame with no marker is
the clause's own default. The third applies the default in the entry's place. What is owed is
`decode_jpeg` reading the entry out of `/DecodeParms`, and the census says what that costs: one
`if`, and no page in 66 918 documents that draws differently for it.

**The lesson is about the instrument, not the clause.** ADR 0036's count was of a *name*, and a
name has a place. A census that reads a parameter out of the dictionary the standard puts it in
answers a different question from one that finds the bytes — and here the two answers differ by
the whole of a decision that stood for a thousand sessions.
