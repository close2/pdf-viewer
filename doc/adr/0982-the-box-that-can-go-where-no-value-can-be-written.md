# 0982 — The box that can go where no value can be written

Session 971. Status: **accepted**. It **overturns ADR 0965's verdict** on the two JPEG 2000
colour-box rows, builds the rewrite that verdict had ruled out, and **re-examines and upholds**
the two duplicate-profile rows against machinery that had arrived since they were refused.

Context: `crates/pdf-transform/src/archive/jpeg2000.rs` (new),
`crates/pdf-transform/src/archive/{decision,prepare,rewrite,mod}.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-transform/tests/archive.rs`,
`doc/pdf-a-mitigations.md` §4.2, §4.5, §13.1, §13.3, §13.3.1,
`doc/pdf-a-conversion-limits.md` §4.10, ISO 19005-2 6.2.8.3, ISO 19005-4 6.2.4.2, 6.2.4.4 and
6.2.7.3, ISO 32000-2 §7.4.9, §8.6.5.7 and §8.6.7, ISO/IEC 15444-1:2000 I.4, I.5.3 and I.5.3.3.

## 1. The question two rounds asked, and the one they did not

ISO 19005-2 6.2.8.3 and ISO 19005-4 6.2.7.3 state two requirements about a JPEG 2000 image's
colour that a file can fail:

- where the data states more than one colour space specification, exactly one shall carry an
  `APPROX` of `0x01`;
- the `METH` a `colr` box states shall be `0x01`, `0x02` or `0x03`.

`doc/pdf-a-mitigations.md` §4.5 answered both twice. The first reading said the fix was cheap
because the fields are in the JP2 wrapper rather than the codestream — true, and about the *cost*.
ADR 0965 corrected it: the cost settles nothing about the **value**, because a `METH` outside the
three the part admits describes this image's colour in a way the part does not read, so writing
one of the three states a colour space the box did not; and marking one specification best
available where the file marks none ranks two of the producer's own statements on evidence the
file does not carry. Both readings are right, and both are answers to one question — *what value
may this converter write into a `colr` box?*

**Neither asked whether a box can go.** The sentence immediately after the method's, in both
parts, is what answers that: a conforming processor shall use only the selected colour space and
shall ignore all the other colour space specifications. The specifications that fail these rules
are, by the target's own subclause, the ones a processor is required to ignore. Removing them
writes no value at all.

That is the correction, and it is a correction about this catalogue's habits rather than about
JPEG 2000: **"there is no mitigation" and "there is no *lossless* mitigation" are different
claims.** RFC 0007 section 2's `discard` had been in the vocabulary throughout. §4.5's four
answers read *Mitigation — none* and *From a configuration — nothing*, and that second line is
where the conflation showed.

## 2. Which box stays, decided by two clauses in two documents

The rewrite chooses nothing. Two sentences select the surviving specification and neither is
ours:

- **Exactly one box carries `APPROX` `0x01`.** Both parts' NOTE 2 makes that value the mark of the
  colour space with the best colour fidelity available, and ISO 32000-2 §7.4.9 sends a processor
  to the same box: "If multiple colour space specifications are given in the JPEG 2000 data, a PDF
  processor should attempt to use the one with the highest precedence and best approximation
  value."
- **Every box states `APPROX` zero.** ISO/IEC 15444-1:2000 I.5.3.3 reserves that field, requires
  it to be zero, has a conforming reader ignore its value, and says in the same clause that a
  conforming JP2 reader ignores every Colour Specification box after the first. The first box is
  then the used one by the core part's own rule.

**The second of those is the interesting one, and it is the round's sharper finding.** A JP2 file
written to part 1 as part 1 requires — `APPROX` zero on every box — is *precisely* the file that
fails ISO 19005's "exactly one marked best", for no other reason than that ISO 19005 gives a
meaning to a field the core part reserves. So the clause that makes the row fail and the clause
that makes it fixable are in different documents. `CLAUDE.md`'s rule that a clause is not read
until its neighbours are reaches one step further than it says: **the neighbour can be in the
standard the clause delegates to.**

Any other shape is refused with its own sentence: two boxes marked best, or a mixture of marked
and unmarked ones, is the file ranking its producer's specifications in a way neither sentence
settles, and choosing there would be ADR 0965's refusal made silently.

## 3. Why it is an authorised loss and not owed

§7.4.9 does not only recommend a box. It states a fallback: "If the colour space is given by an
unsupported ICC profile, the next lower colour space, in terms of precedence and approximation
value, shall be used. If no supported colour space is found, the colour space used shall be
DeviceGray , DeviceRGB , or DeviceCMYK , depending on the whether the number of ordinary channels
in the JPEG 2000 data is 1, 3, or 4." ISO/IEC 15444-1:2000 I.5.3 says the same thing from the
producer's side — several specifications are the file's compatibility and optimisation options.

So the removed boxes are a chain. A processor that can use the box that stays sees no difference
whatever; one that cannot now falls back to a device space instead of to the producer's second
choice. That is small, real and nameable, which is `Answer::Loses` — `Loss::Jpeg2000ColourFallback`,
`--authorise jpeg2000-colour-fallback`. It is emphatically **not** `doc/pdf-a-mitigations.md`
§13.3's *owed, not optional*, and §13.3 says so rather than growing a row.

## 4. What the rewrite declines to touch, and the proof it keeps

Removing bytes from inside the `jp2h` superbox moves every byte after it, so the edit is bounded
by what this project can say about the boxes it is moving:

- **Only part 1's boxes.** ISO/IEC 15444-1:2000 Annex I defines no box that states a byte offset
  into the file. ISO/IEC 15444-2 — which defines the fragment tables that do — is not held here
  (`doc/questions/A51`), so a file carrying a box part 1 does not define is refused rather than
  guessed at. The top-level and `jp2h` box-type lists in `jpeg2000.rs` are Table I-2's, and a type
  outside them stops the edit.
- **Only a stream whose bytes *are* the JPX data**, so `/Filter` must be `JPXDecode` alone and the
  stream must state no `/F`.
- **Only a surviving box whose `METH` the part admits**, which is why a file with a single
  offending specification is refused: there is nothing to remove, nothing for a processor to fall
  back to, and the only route left writes a value.

And the proof, which is ADR 0973's rule applied to bytes rather than to a dictionary entry: the
rewritten data is parsed again before it is promised, and the document is refused unless exactly
one specification survives, it is the one selected field for field, its method is admitted, and
every other header the data states — image header, per-component depths, channel definitions,
component mapping, the codestream's `SIZ` marker — is the one it stated before.

**The corpus witness settles what that is worth.** `veraPDF test suite 6-2-8-3-t02-fail-a.pdf`
converts to PDF/A-2b where it was refused before, the output is held to the target again and
conforms, and the page rendered from the output is **byte-identical** to the page rendered from
the source. Its neighbour `6-2-8-3-t03-fail-a.pdf` states one specification with a forbidden
method and is refused, with the sentence saying exactly why.

## 5. The duplicate-profile pair, re-examined and upheld

ADR 0965 refused `graphics/no-icc-space-duplicating-the-output-intent-profile` and
`graphics/separation-alternate-space-does-not-duplicate-a-current-profile` on two grounds: the
failure is not sited on the array that would be rewritten, and the substitution would not be a
restatement. ADR 0973 then built `Rewrite::SpotColorantEntry`, the first rewrite in this crate to
reach an object that *is* a colour space array — so the siting ground was worth asking again, and
this round asked it.

**It does not change the verdict, and the reason is one sentence.** `SpotColorantEntry` reaches an
array because its own finding **names that array's object**; `sites::colorant_entries` refuses
outright when `finding.place.object` is absent. These findings name the content stream that
*used* the space. Run against the corpus witness the finding reads

> page 1, ICCBased: a colour space operator uses an ICCBased CMYK colour space whose profile is
> the profile in the PDF/A output intent then current

— a page, a name of `ICCBased`, no object. The array is still in a resource dictionary nothing
points at, and siting it would mean walking the content streams a second time to decide which
space was used, which is the validator's reading made again in the converter.

**The second ground is stronger than it was written.** ADR 0965 said the substitution "can decide
a composite §8.6.5.7 leaves open". §8.6.5.7 says more than that: "The conditions under which such
implicit conversion is done cannot be specified in PDF" and the conversion "is completely hidden
by the PDF processor and plays no part in the interpretation of PDF colour spaces." So whether an
`ICCBased` CMYK space behaves as `DeviceCMYK` — and therefore whether §8.6.7's non-zero overprint
mode applies to it — is a decision the base standard assigns to the processor. Writing
`/DeviceCMYK` into the file takes that decision away from the processor and settles it in the
file, which is not a restatement under any reading and is the very ambiguity 6.2.4.2's NOTE 2
names as the reason for the prohibition.

Six corpus documents are refused on this row at PDF/A-4 today, so it is not an idle one. The
refusal sentence now carries both halves in the form above.

## 6. The general lessons

1. **An entry whose answers include *From a configuration — nothing* deserves a second reading.**
   That line means the catalogue believes no operator could authorise anything here, and it is
   where a "no lossless route" quietly became "no route".
2. **A clause's neighbours can be in another standard.** ISO 19005's colour-box rules are only
   fixable because ISO/IEC 15444-1 says which box a reader uses; ISO 19005 never says.
3. **A rewrite reaching a new *shape* of object does not site a failure reported somewhere else.**
   What sites a rewrite is the finding naming the object, and nothing else — which is worth
   stating once, because "the machinery now exists" is exactly the kind of claim that gets a
   refusal overturned for the wrong reason.
