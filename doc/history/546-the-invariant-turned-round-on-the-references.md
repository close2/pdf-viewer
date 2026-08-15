# 546 — The invariant turned round on the references, and the family member outside it

**Finding.** The oracle's **contradicted** bucket, ranked the way `doc/habits.md` asks — our worst
measurement over the bound it is held to — is headed by the seven JBIG2 pages, whose group's whole
argument is *negative*: `mupdf` and `ghostscript` are one decoder, so their agreement is not
evidence. That says nothing about who is right. **The corpus's own invariant is what says it, and it
had only ever been pointed at us**: the pdf.js `bitmap-*` family is one drawing encoded through
nearly every path ISO/IEC 14492 defines, so *every* program owes the same picture on all of them and
each can be compared **with itself** — no renderer treated as truth, principle 5 untouched. Asked
that question, this tree returns one image; `poppler`, `mupdf` and `ghostscript` return eight, six
and six. And the image `jbig2dec` produces on the encodings it *is* consistent about is
byte-identical to ours (`magick compare -metric AE` = 0, against both programs).

**Second finding: the population had a hole, and the hole was on the contradicted list.**
`tests/jbig2.rs` built its documents from the `bitmap-` filename prefix. `issue20439.pdf` is the same
drawing — our render of it and of `bitmap-halftone-composite.pdf` differ in **zero** pixels — through
one `/JBIG2Decode` XObject on the same page box, and it was outside the one instrument in this tree
that can judge a JBIG2 decode while the oracle listed it as contradicted. **A population defined by a
naming convention excludes the members named after something else, and those are likelier to be
interesting**: a file named after an issue is a file somebody filed an issue about.

**Third finding: a reference's silence is a fact about the invocation before it is a fact about the
renderer.** `CONTRADICTED_REFERENCES_DREW_NOTHING` says `mupdf` and `ghostscript` "both say, in
words, that they threw the object away". Under the oracle's own command line `gs` says **nothing** —
`-q` — so a round checking that note against the gate's invocation would have found silence and
called it wrong. Without `-q` it names them. The same shape one door over: six of the seven JBIG2
pages produce no warning from either program under any flag, and `jbig2dec` simply returns a
different picture.

**Date.** 2026-08-15.
**ADR.** [0381](../adr/0381-the-invariant-turned-round-on-the-references.md).

## The pages diagnosed

(a) our defect · (b) their shared gap or shared code · (c) genuinely unspecified. The rank is by
our worst measurement over the bound it is held to, over all 67.

| rank | page | prior label | verdict | evidence |
|---|---|---|---|---|
| 2 | `bitmap-halftone-composite.pdf` p1 | `CONTRADICTED_SHARED_JBIG2_DECODER` | **(b), verified and sharpened** | `mupdf` ≡ `gs` byte-identical (AE 0), ink 22.594 against our 17.495; **both silent**; our render ≡ their own render of `bitmap-halftone.pdf` (AE 0) |
| 4 | `issue20439.pdf` p1 | same | **(b), and a population hole** | ours ≡ our `bitmap-halftone-composite.pdf` (AE 0); `mupdf`'s ≡ its `bitmap-symbol-texthuffrefinecustom.pdf` (AE 0); references blank |
| 5 | `bitmap-symbol-texthuffrefinecustomposdims.pdf` p1 | same | **(b), verified** | `mupdf` ≡ `gs` blank (ink 0.000), both silent |
| 6 | `bitmap-symbol-texthuffrefinecustom.pdf` p1 | same | **(b), verified** | as above |
| 7 | `bitmap-symbol-context-reuse.pdf` p1 | same | **(b), and the pair is not the usual one** | `mupdf` ink **255.000** (black page), `gs` **0.000**; AE between them 159 600, every pixel. The only one of the seven where `jbig2dec`'s two hosts differ, and the only one that warns |
| 8 | `postscript_type4_many_outputs.pdf` p1 | `CONTRADICTED_DEVICE_CMYK_CONVERSION` | **(c), re-sampled** | at `c` = 0.5: ours (127, 214, 247), `poppler` (128, 214, 247), `mupdf` (109, 207, 246), `gs` (108, 207, 246), `hayro` (109, 206, 246) — 127 is 255 × (1 − c) exactly |
| 9 | `bitmap-refine-page-subrect.pdf` p1 | `CONTRADICTED_SHARED_JBIG2_DECODER` | **(b), verified** | `mupdf` ≡ `gs` 21.052 against our 17.495, both silent |
| 10 | `bitmap-symbol-symhuffrefineone.pdf` p1 | same | **(b), verified** | `mupdf` ≡ `gs` 19.422, both silent |
| 12 | `issue14802.pdf` p1 | `CONTRADICTED_LINK_BORDER` | **(b), verified and extended** | ours 546 px and `poppler` 550 px of exactly `#0000FF`; `mupdf`, `gs` **and `hayro`** draw none; step 7 gap **+10.001**, the whole list's largest positive |
| 14 | `issue11549_reduced.pdf` p1 | `CONTRADICTED_REFERENCES_DREW_NOTHING` | **(b), verified, note corrected** | `mupdf` "ignoring broken object (70 0 R)"; `gs` silent under `-q`, and without it "object lacks an endobj", "xref table was repaired", and it loads a substitute face |
| 15 | `file_url_link.pdf` p1 | `CONTRADICTED_LINK_BORDER` | **(b), verified** | all three references silent — the Print flag is a decision `gs` takes without a word; step 7 gap +3.051 |
| 17 | `issue7115.pdf` p1 | same | **(b), verified** | as above; step 7 gap +2.836 |
| 18 | `issue11740_reduced.pdf` p1 | `CONTRADICTED_REFERENCES_DREW_NOTHING` | **(b), verified, note corrected** | `poppler` "Mismatch between font type and embedded font file"; `gs` without `-q` "error reading a stream" and a substitute face; step 7 gap +13.704 |

**Thirteen pages, and not one label was wrong** — which is the first time a sweep of this bucket has
ended that way, and is worth reporting as itself rather than as nothing. Trap 1's tally is
unchanged at ten for ten. What *was* wrong was evidence inside three of the notes, under diagnoses
that hold: a log generalised from one page to seven, a claim about `ghostscript`'s log that needs a
flag the gate does not pass, and a fourth renderer's silence nobody had recorded.

## The instrument, and why it is not a gate

```sh
# the family, through §2's own reference command lines, grouped by the picture
pdftoppm -r 72 -png -f 1 -l 1 -singlefile -cropbox -aa yes -aaVector yes <pdf> <out>
mutool draw -b CropBox -r 72 -o <out>.png <pdf> 1
gs -q -dNOPAUSE -dBATCH -dSAFER -sDEVICE=png16m -dUseCropBox -r72 \
   -dGraphicsAlphaBits=4 -dTextAlphaBits=4 -dFirstPage=1 -dLastPage=1 -sOutputFile=<out>.png <pdf>
magick <png> -depth 8 -colorspace Gray txt:- | sha256sum     # one hash per rendered picture
```

About 300 reference renders, and it measures somebody else's decoder rather than ours — so it is
written into the group's note, the ledger's §7.4.7 row and ADR 0381 with the recipe beside it,
rather than made a gate. What is gated is the half that is about us, and that half now covers 97
documents instead of 96.

`poppler` cannot be compared byte-wise with anyone: it smooths the image on the way to the page,
198 grey levels against our two at one device pixel per sample, which is exactly the 17.589 against
our 17.495 that appears on five of the seven pages. Its self-consistency is still measurable and it
fails the invariant on 17 of the family.

## Gates, verbatim

```text
cargo fmt --all --check                                   clean
cargo clippy --workspace --all-targets                    silent of lints
cargo nextest run --workspace                             2013 tests run: 2013 passed, 15 skipped
cargo test --workspace --doc                              1 passed, 0 failed
corpus    974 documents in 3.1s: 0 unopenable, 8 locked, 2 encrypted beyond us,
          6 pageless, 64 incomplete, 0 slow
oracle    1794 pages in 49.3s (1691 complete, 103 incomplete)
          agrees 906/862   contradicted 67/66   ambiguous 786/753
          our geometry 1/0   reference geometry 2/2   not comparable 13/8   no render 19/0
text      974 documents in 34.9s: 25 skipped, 61 incomplete and not gated;
          overall 99.2% (22836/23015 words), 22 below 90%
          10969/11163 matched words in bounds (98.26%), 486 of 508 documents fully in bounds
dates     1545 strings, 1514 conform to §7.9.4 (97.99%)
xmp       319 documents carry §14.3.2's stream, 318 read, 3191 properties
jpeg2000  14 codestreams byte-identical to OpenJPEG's decode
quorra    956 pages in 29.4s: 931 agree, 23 differ, 2 refused, 18 not comparable
quorra    gpu lane at scale 4: 951 pages in 261.9s: 937 agree, 10 differ, 4 refused,
          23 not comparable (ratchets off, and the run says so)
conformance  5 tests pass; 8404 citations, 800 quotations all verbatim
jbig2     97 JBIG2 encodings, 399x400, all identical
```

**No pixel moved, and it is checked rather than asserted.** The oracle was run before the round's
diff and after it: its **888 per-page lines are byte-identical**, and every tally with them. Nothing
in `crates/*/src` changed — the diff is one test's population, five doc comments, a ledger note and
four documents — so `doc/todo/00` step 7 over the *ambiguous* bucket is not owed. It was run over
the **contradicted** list anyway, as a check on eleven sessions of tree movement underneath: the
head is `issue4436r.pdf` −2.203, `issue9243.pdf` −1.549, `smask_luminosity_oob_transfer.pdf` −0.778,
`issue7580.pdf` −0.485 and nothing else past −0.4 — the same names in the same order as
`doc/oracle-and-corpus.md` §3b records, with `issue5751.pdf`'s −5.115 gone because the
five-hundred-and-fourteenth fixed it. The positive side is `issue11740_reduced.pdf` +13.704 and
`issue14802.pdf` +10.001, both a reference that drew nothing. Nothing unexplained.

**Not done, and why.** `doc/todo/02` §5's binaries were not rebuilt: nothing a person runs changed.
The reference cache was the main checkout's, read and written, on session 514's argument — 99.8% hit
rate, which is trap 10a's tell reading correctly.

**Touched.** `crates/pdf-model/tests/jbig2.rs` (the population, the ratchet, and the module comment's
new section), `crates/pdf-model/tests/oracle.rs` (four group notes: the JBIG2 measurements and the
reference invariant, the link border's three additions, the drew-nothing pair's `-q` correction, the
`DeviceCMYK` ramp re-sampled), `doc/conformance/ledger.toml` (§7.4.7), `doc/HANDOVER.md` (trap 9),
`doc/oracle-and-corpus.md` (§3b, §3c), `doc/adr/0381-*` (new), this file.
