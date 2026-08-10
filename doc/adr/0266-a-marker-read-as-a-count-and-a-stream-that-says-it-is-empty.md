# ADR 0266 — A marker read as a count, and a stream that says it is empty

Date: 2026-08-11 (session 430)
Status: accepted

## Context

ADR 0261 established that a `CC-MAIN-2021-31` archive is a **hash bucket** — the whole crawl sorted
by SHA-256 and cut into 7933 equal pieces — so depth costs nothing in representativeness and the
182 KiB of central directory per archive is the only thing breadth pays extra for. `doc/todo/03`
carries the rule that followed: take whatever is cheapest among what nobody has taken.

Sessions 426 and 427 then closed §11.4.7's page-group blending space, which had been **67 of the
86 reports** in that sample and the largest correctness gap this tree had against real files. This
round is the first look at the web with that one closed, and the question it can answer is what
the *residue* looks like — which is a fact about this tree that nothing but a corpus can supply.

## Decision 1 — the sampling rule, and what it cost

**Four whole archives, `0100 + 2000k` for k = 0 … 3: `0100`, `2100`, `4100`, `6100`, every member
of each.** Four properties, each the reason for a part of it:

- **Whole archives rather than windows**, which is decision 2 of ADR 0261 spent rather than
  re-argued: 4000 documents for **four** directory reads where session 425's stride paid 79 for
  1896. The four directories were 182.2 KiB apiece, **728.7 KiB in total against 14.1 MiB**.
- **≡ 100 (mod 1000)**, which is disjoint from session 425's ≡ 50 (mod 100) stride and from
  `0000` and `3500`, and misses the thousand-boundaries where the corpus changes directory.
- **A stride of 2000 across the range**, so the next round extends the rule by moving the offset
  — `0300 + 2000k`, then `0500` — rather than deciding again.
- **The choice of *which* archives is immaterial and that is the point.** Nothing about a document
  can correlate with its content digest, so this rule is reproducible rather than justified: any
  four archives are the same sample, and a rule is what stops two rounds fetching one archive
  twice.

**What it cost: 5409.6 MiB of member byte ranges and 728.7 KiB of central directories, 5.04 GiB in
total, for 4000 documents.** Four fetches, 0 failures, every member verified against the CRC-32
its archive records, 105 + 131 + 90 + 112 = **438 s** of wall clock. Nothing was committed.

**Running total: 7.72 GiB over 85 archives and 5944 documents** — 15.4% of the owner's 50 GB read
the way ADR 0261 read it, 16.6% counting decimal gigabytes. The promotion budget is untouched at
**0 MB**: both witnesses below are named by archive and digest, and both fixtures are generated.

## Decision 2 — what the residue is, ranked

**4000 documents in 53.3 s: 6 unopenable, 3 locked, 2 encrypted beyond us, 2 pageless, 70
incomplete, 0 slow**, with 1161 codes reaching no glyph in silence over 33 documents. A baseline
for this chunk, never a ratchet.

**The rate more than halved and the reason is last round's work.** 86 of 1896 was 4.5%; 70 of 4000
is **1.75%**, and §11.4.7's population went from 67 documents to 24 with the conversion into the
blending space built (ADRs 0262, 0263). Nothing else changed between the two samples.

**Nothing failed to open for a reason that is this tree's**, for the second sample running. All
six unopenable documents carry no `%PDF-` header in their first kilobyte — they are HTML saved
under a `.pdf` name — and the two pageless ones and two of the three undecodable content streams
are the same crawl artefact ADR 0261 named: a PDF the origin server truncated at about a kilobyte
with a Baidu link-submission script where the body should be. The two encryption refusals are the
standard's own edges: an `/Encrypt` that does not resolve to a dictionary (§7.6.1) and `/R` 5,
"a deprecated proprietary extension the standard states no algorithm for" (§7.6.4.2, Table 21).

The 70 reports, by the population each belongs to — a document reporting two things is counted in
both, so the column sums past 70:

| population | documents | already named by |
|---|---|---|
| §11.4.7's page-group blending space | **24** | `doc/todo/23`, ADR 0251 |
| a font whose program has no outline for any code the page shows | **14** | `doc/todo/21` §3 |
| an image | **11** | below |
| a budget stopped interpretation | **8** | nothing, until this round |
| §11.6.6's group in a space of its own | 4 | `doc/todo/23` |
| §11.4.4's non-isolated group with an element that blends | 3 | `doc/todo/23`, ADR 0234 |
| text-showing operators skipped | 3 | with the font rows |
| a `/Contents` part that would not decode | 3 | decision 4 |
| six singletons | 1 apiece | §7.8.3, §11.4.6, §9.3.8, a shading, an operator, an annotation |

**The first row is not the row it was, and the split is the result.** All 24 are refused by a
*named* condition since ADR 0262, and the conditions are not one population:

| what the page states | documents |
|---|---|
| an array-formed space whose four components are not `/DeviceCMYK` | **10** |
| the document names the press its `DeviceCMYK` is (§8.6.5.6, §14.11.5) | 8 (4 of them array-formed) |
| a group inside the page composites in a different space (§11.6.6) | 4 |
| a non-separable blend mode, whose black component has a rule of its own (§11.3.5.3) | 2 |

**Every one of the 14 array-formed page groups is a four-component `ICCBased` space**, checked
with `examples/group_space_census` rather than assumed. `doc/todo/23` prices that row at "its
conversion out is a profile rather than sixteen corners" and gives it **0 corpus documents and 1
web witness**; it is **14 of 4000, 0.35%**, and it is the largest single named residue this sample
has. That is the row to take next and this round did not take it — an ICC `B2A` transform is a
session of its own — but it is now a number rather than a corner.

**The image row is five things and two of them were this round's**, which decisions 3 and 4 took:
4 documents whose JPEGs were refused whole, 4 with an `/SMask` carrying a `/Matte`, and one apiece
for a `/Mask` at a grid too large, a `CCITTFaxDecode` whose `/Columns` is not the image's width,
and a JPEG whose first bytes are not a marker.

**And the budget row is a population nobody had named.** `MAX_TILES` on four documents and
`MAX_OPERATIONS` on four, **8 of 4000 — 0.2% of the web** — against one of each in session 425's
1896, so the rate is stable across two samples. `doc/todo/49` lists these budgets under "keep, and
they are not negotiable", and this changes none of that: what it adds is the number that was
missing from the argument, and **not one of the eight was slow**, so the bound stops the work
inside the per-document budget rather than after it. Whether the eight pages are *right* is the
question, and it is `doc/todo/03`'s for a later round. Raising a constant because a corpus reached
it would be exactly the move `CLAUDE.md` forbids.

## Decision 3 — a `DCTDecode` frame's component count comes from the frame

**Four documents lost 21 images whole**, each reported
`malformed image: JPEG data: "Unimplemented colorspace mapping from RGB to CMYK"`, and the defect
is this tree's.

`decode_jpeg` decided whether a codestream had four components from `zune-jpeg`'s
`input_colorspace()`, read straight after `decode_headers()`. That value is the decoder's reading
of Adobe's APP14 marker, and APP14 transform 0 maps to `CMYK` there **whatever the frame says** —
the library's own comment records why, and records that it defers the correction:

> in case of adobe app14 being present, zero may indicate either CMYK if components are 4 or RGB
> if components are 3 … so since we may not know how many number of components we have when
> decoding app14, we have to defer that check until now.

"Until now" is inside `decode()`, one call after this tree had already asked for four channels out.
Asking a three-component frame for `CMYK` is a conversion `zune-jpeg` does not have, so the image
was refused rather than drawn.

**The standard says where the number lives**, §7.4.8:

> The values of these parameters, which include the dimensions of the image and the number of
> components per sample, are entirely under the control of the encoder and shall be stored in the
> encoded data.

and Table 13 states the transform *in terms of* that number — "If the image has three colour
components, RGB values shall be transformed to YCbCr before encoding and from YCbCr to RGB after
decoding. If the image has four components, CMYK values shall be transformed to YCbCrK before
encoding and from YCbCrK to CMYK after decoding" — so transform 0, "No transformation", is a claim
about what was applied and not about how many components there are. The count is now
`info.components`, which is `SOF`'s own byte.

**The `YCCK → CMYK` conversion is gated on the same test**, and that is the second half rather than
tidiness: a *three*-component frame whose marker says transform 2 is read as `YCbCr` by the
decoder — its own workaround for a malformed file — and running `ycck_to_cmyk`'s
`chunks_exact_mut(4)` over three-channel pixels would walk across every pixel boundary and produce
colours rather than a refusal.

**What the witnesses are.** All 21 are three-component frames carrying an Adobe APP14 marker with
transform 0; the marker combination was read out of each file's `SOF` and `APP14` segments rather
than inferred from the failure. `0100720.pdf` is 10 of them on page one, `2100465.pdf` 3,
`6100103.pdf` 3 and `6100775.pdf` 5. Named, not committed:

| document | archive | SHA-256 |
|---|---|---|
| `0100720.pdf` | `0100` | `034411f89a56b3ae33717791ae15b51def0b97e674845df6ca417cc6b40eda60` |
| `2100465.pdf` | `2100` | `43b9e0ae545ee3892652e7b0f416532834c19c71591b4c5ff2c3b9a7d58adc41` |
| `6100103.pdf` | `6100` | `c4d786c1450fe89a666f8a9abb950a61cdd6fe1056eeac5076e723cc090dadfd` |
| `6100775.pdf` | `6100` | `c4dcde8278f4f7909d70bbd23acb4dd339250f392065ac914c699e895ae72cd2` |

`crates/pdf-model/tests/dct_components.rs` is the regression test, and its fixtures are **written
out rather than encoded**: an 8×8 baseline frame with an identity quantisation table, one DC
category zero and an immediate end-of-block per component, which is 170 bytes and reproduces the
decoder's message exactly at the pre-fix commit. Three tests — transform 0, transform 2, and the
same frame with no APP14 marker as the control that says the marker was what decided.

## Decision 4 — a stream the file states is empty has nothing to decode

`doc/todo/03` §5 carried this from session 425 with the diagnosis and without the argument, and the
argument is what it needed. A `/Contents` part written
`<< /Filter /FlateDecode /Length 0 >>` was reported `Undecodable`, because `flate` refuses an empty
input — RFC 1950 gives a zlib stream a six-byte floor. The report claims the page is missing
drawing, and an empty part cannot be.

**Two clauses decide it and they decide different halves.** §7.3.8.1 makes the stream conforming:

> A stream shall consist of a dictionary followed by zero or more bytes bracketed between the
> keywords stream (followed by newline) and endstream :

and Table 5 has `/Filter` name the filters "that shall be applied in processing the stream data
found between the keywords stream and endstream " — with no data found there, nothing is applied
and the decoded result is the empty sequence. That is not the filter succeeding on an empty input;
it is there being no input to filter.

**The objection session 425 raised is real and §7.3.8.2 answers it.** A stream *truncated* to
nothing arrives holding no bytes exactly as an empty one does, because `pdf-syntax` recovers a
wrong `/Length` by searching for `endstream` — so "the bytes are empty" cannot tell the two apart
and a rule built on it would silence a real loss. This clause can tell them apart:

> Every stream dictionary shall have a Length entry that indicates how many bytes of the PDF file
> are used for the stream's data.

with "[a]ll of these constraints shall be consistent" one sentence on. A truncation leaves
`/Length` stating bytes that are not there; only a stated zero the bytes agree with is silence the
producer asked for. `Document::states_no_data` is both halves and nothing less.

**Deliberately not on `Document::image_stream`'s path**, and the reason is in the same clause:
"streams are used to represent many objects from whose attributes a length can be inferred", and an
image's `/Width`, `/Height` and `/BitsPerComponent` infer one. For an image a stated zero
contradicts the dictionary rather than agreeing with it, and stays an error.

**Two of the 5944 members on disk do this** — `0100119.pdf` (archive `0100`, SHA-256 `033ea7b40a92448fd6494ec52319600410c6d31fd180961820243e131d9c0b11`) and ADR 0261's
`4150022.pdf` (archive `4150`, SHA-256
`85ea41fdedc0a195deacd9aedf88df9a0e002bd05015940d1e2351c55a1b9c29`) — and a third document,
`0100750.pdf` (SHA-256 `034440c7519a398482cd3e640f0fa0e4e757852b3059fde19a6537091fd39715`), had an *annotation* appearance stream of the same shape. **Two more are truncations
and still report**, which is what the second test asserts: `4100367.pdf` and `4100387.pdf`, both of
them the Baidu artefact, both stating a `/Length` in the thousands with no `endstream` in the file
at all.

## The fuzzer over the new seeds, which is now nearly free

`fuzz/seed_page.py` over every document on disk — the 5944 SafeDocs members, `doc/corpora`'s 108
and the pdf.js submodule's 974 — takes `fuzz/corpus/page` to **8572 seeds**, and
`cargo +nightly fuzz run page -- -runs=50000 -fork=6 -rss_limit_mb=4096 -timeout=60` is **51 324
iterations in 1255 s: 0 crashes, 0 out-of-memory, 0 timeouts.** Coverage over the run went
**32 109 → 32 671** edges with the corpus growing 4704 → 6150, against session 428's 28 535 at
seeding — so 4000 new documents were worth about 3600 edges to a target that had already been run.

**Two new `slow-unit-` artefacts, and ADR 0264's rule says read them in a release binary before
believing them.** Both are 186 350 bytes; one *is* a corpus member — `6100150.pdf`, archive `6100`,
SHA-256 `c4d7f72986d5c24edd2b9f1a4fa9aaa9eb647412baaa28d1e55c43ff3eb9dfe2` — and the other is a
mutation of it. `target/pdf-retrieve page … 0` runs each in **0.185 s** and **0.189 s**, so they are
the sanitiser's slowness and not the product's, exactly as session 428's five were. Nothing was
promoted and no budget was touched.

## Consequences

- **The chunk's survey moves 70 → 64 incomplete** on this round's own work: four documents' JPEGs,
  one zero-length content stream and one zero-length appearance stream. Everything else is
  unmoved, including the 1161 silent codes, which nothing here touches.
- **The 974-document corpus gate is unmoved at 68 incomplete, and that is *proved* rather than
  inferred.** A count that does not move is not evidence that nothing happened — `doc/todo/02` §7's
  second habit — and this round had a real reason to check: a scan of the 974 finds **16 documents
  with a three-component JPEG carrying an Adobe APP14 marker** and **8 whose stream dictionaries
  pair a `/Filter` with `/Length 0`**, one of them named `multiple-filters-length-zero.pdf`. So
  `examples/display_list_digest` was run on this tree and in a detached worktree at `c1c9e62`, and
  the two files are **identical over 975 lines**: no corpus document's page-one display list moved.
  The sixteen JPEGs are transform 1, which was never on the changed branch, and none of the eight
  zero-length streams is a first page's content or a first page's appearance. `doc/todo/00`'s step 7
  is therefore **not owed**, on an artefact rather than an argument.
- §7.4.8 keeps `partial` and gains the reading; §7.3.8.1 and §7.3.8.2 keep `implemented` and gain
  theirs. No row's status moved.
- **The four-component `ICCBased` page group is the named row with the largest web population**,
  14 of 4000, and `doc/todo/23` carries it with that number.
- **The budget population is named for the first time**, 8 of 4000, in `doc/todo/03`.
- The 50 GB budget is **15.4% spent**; the promotion budget is **0 MB spent**.
