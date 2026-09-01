# 857 — The trailer a rebuild could not find

2026-09-01. ADR 0781. `crates/pdf-syntax/src/xref.rs`,
`crates/pdf-syntax/tests/cross_references.rs`, `doc/checks/fixed-documents.toml`,
`doc/conformance/ledger.toml`, `crates/pdf-model/src/content.rs`,
`doc/todo/03-more-corpora.md`, `doc/traps/parsers-and-streams.md`.

The primary item was 855's remainder: read the unopened documents of the Tika issue-tracker chunk
against §7.5, and either say per clause why a conforming reader cannot open them or fix what
poppler's recovery reads and ours does not.

## The population was not what 855 said, and neither was the instrument

855 recorded "sixteen documents these four directories could not open or found no page in", of
which "eight get a page count out of `pdfinfo`", two read by hand and six unread. Both numbers are
wrong, and the second is wrong in a way worth more than the first.

**The population is 28**, which is what the four survey lines 855 printed already add up to. **13
of the 28 get a page count out of `pdfinfo`**, not eight.

**And a page count is the wrong question.** `pdfinfo`'s `Pages:` is the page tree's `/Count`, and
this tree prints the *same* number for ten of the thirteen, because `Pages::len` reads `/Count`
too. The question that discriminates is whether poppler can *draw* page one, and asked that at
36 dpi it answers a 1×1 image or a blank US-Letter sheet for eleven of the thirteen — the `/Kids`
those trees name are objects the bug reports' own reductions removed, so neither reader has a page
and neither is wrong about the file.

That is trap 9's shape wearing a different hat: a reference claim taken off an instrument that was
answering a different question. The correction is in `doc/todo/03` §29 with the instrument named
beside it, which is what the sentence lacked.

## The two that carry ink

`PDFBOX-4777-0.pdf` and `PDFBOX-4777-1.pdf` — two attachments to the same tracker, both PDF 1.6
written entirely with cross-reference streams, both **encrypted**, and both with a `startxref`
nineteen bytes short of their cross-reference stream's object header.

So the address cannot be followed, and §C.4's rebuild takes over: scan for `N G obj` headers, look
for a trailer by searching for the `trailer` keyword, and failing that insert a `/Root` naming the
first object that declares itself a catalogue. §7.5.8.1 is why the middle step finds nothing —
"the keywords xref and trailer shall no longer be used" in such a file — so the recovered trailer
was `/Root` and nothing else. **`/Encrypt` went with the rest of Table 15**, `is_encrypted()`
answered false, and every string and stream came back as ciphertext: the catalogue's `/Lang` as
thirty-two random bytes, every object stream as an unknown compression method, no page tree. The
document opened, reported nothing, and drew nothing — trap 5's failure in its purest form, a
plausible answer rather than a refusal.

§7.5.8.2 says where the trailer went, and it is quoted in the code: a cross-reference stream's
dictionary carries Table 15's entries in addition to Table 5's and Table 17's.
`find_xref_stream_trailer_by_scan` reads it, taking the stream furthest into the file for
§7.5.6's reason. Both documents now authenticate under §7.6.4.1's default user password and draw:
25 pages and 1, matching `pdfinfo`, **reporting nothing**, at 2474 and 22 384 inked pixels against
poppler's 2571 and 20 317. Both are rows in `doc/checks/fixed-documents.toml`. Over the 4525
documents of the fetched chunk, exactly these two move.

The test is a pair — two pairs — differing **only in the `startxref` address**, because the rule is
that the two answers agree. `/Root` alone does not discriminate, since the catalogue scan finds it
without reading a trailer at all, so one pair asks for `/Info` and the other asks whether an
`/Encrypt` this reader does not implement still refuses. Run against the tree before the change,
the first fails on `/Info`.

## The nine that are not ours, per clause

- `PDFBOX-3870-0.pdf`, `PDFBOX-3894-0.pdf` — `/Count` 5 and 2 over `/Kids` naming objects the file
  does not carry (and 3894's `startxref` is 10 322 into a 2316-byte file). §7.7.3.2 makes `/Kids`
  "an array of indirect references to the immediate children"; there are none. Both readers print
  the count and neither has a page.
- `PDFBOX-4623-1.pdf` — object 2 is a `/Pages` node whose `/Kids` is `[2 0 R]`, itself. poppler
  says *Loop in Pages tree* and draws 1×1.
- `poppler-192-0.pdf`, `poppler-355-0.pdf`, `poppler-742-0.pdf`, `poppler-750-0.tgz-0.pdf` — kids
  present in the body but mutated past parsing; both readers count and neither draws.
- `cairo-274-0.pdf` — the catalogue's own object header is `1 0 Ybj`. §7.3.10's `obj` keyword is
  not there, so the header scan cannot attribute the object and `/Root` resolves to nothing.
- `cairo-51-0.pdf` — object 1 writes `/Type` as `\x1dType`; same outcome by a different mutation.

Recovering either of the last two means accepting a keyword the standard does not write, which is
guessing and not recovery. They stay refused, out loud.

## What was diagnosed and deliberately not taken

`Pages::new` runs `scan_for_pages` — the recovery that finds a page by Table 31's own `/Type /Page`
declaration — under the guard `count == 0`, with a comment saying it "runs only where the tree
produced nothing". Those are different tests, and four documents of this chunk carry exactly one
self-declaring page the tree cannot reach, `PDFBOX-4623-1.pdf`'s *Hello World* among them. The
guard is one condition; what stopped it is that changing it forces an answer to what `len()` means
when a tree contradicts its own `/Count`, and inventing that at the end of a round is the shortcut
principle 1 forbids. `doc/todo/03` §31 has the four witnesses, the clause and the ordering the
startup rule imposes on the two conditions.

## The archives

Six attempts apiece at `batch2`–`batch5`, each starting from nothing, and the Archive answered
almost all of them with a `504` or `502` page within a second. **`batch3` landed on the fifth
attempt** — 3 606 571 824 bytes matching Apache's published SHA-512 exactly. The other three did
not: `batch5`'s six attempts were all the 160-byte error page, and `batch2` and `batch4` each got
one transfer that started and then stopped short.

Two things follow, and the second is a hypothesis rather than a finding.

**855's suspicion that the Archive's copy of `batch2` "may be short" is retired.** Its two
truncated attempts stopped at two *different* lengths, 4 201 315 360 and 4 201 109 128 — a
connection dropping, not a file ending — and `batch3` came back byte-exact through the same
throttling. `HEAD` on `batch4` answers 302 naming its capture, so the captures are all there. What
the evidence supports is rate limiting, and what to change is the number of attempts rather than
the recipe: the file that succeeded did so on its fifth try.

**And a thing to test rather than to believe.** The only two transfers that ever started on a file
larger than `batch3` both stopped near the *same* size — `batch2` at 4 201 109 128 and `batch4` at
4 201 227 576, against stated sizes of 4.2 and 5.4 GB — while the one that completed is 3.6 GB.
That is n = 2 and could be coincidence, but if it is a transfer ceiling on this route then
`batch2`, `batch4` and `batch5` want HTTP `Range` requests in pieces rather than one `GET`, and
the Archive serves byte ranges (which is how `doc/todo/03` §29's recipe found the corpus in the
first place). A round retrying these should try that before spending another six attempts.

## The chunk: `batch3` is one tracker, and it is the easiest one

`batch3` holds a single directory — `MOZILLA`, the Firefox bug tracker — with **6835 documents**,
more than `batch1`'s whole `PDFBOX` shard set. Surveyed as eight shards, one process each:

| directory | documents | line |
|---|---|---|
| `batch3/MOZILLA` | 6835 | 5 unopenable, 1 locked, 3 encrypted beyond us, 17 pageless, 169 incomplete, 36 slow |

**2.47% incomplete**, against `PDFBOX`'s 7.25%, `poppler-gitlab`'s 11.7%, the pdf.js corpus's
6.98% and the whole crawl's 1.735%. So 855's "a corpus of bug attachments is four to seven times
harder than the web" is a statement about *which tracker*: PDFBOX's and poppler's attachments are
reduced and fuzzed cases somebody built to break a parser, and Firefox's are documents a person
found in the wild and could not read. This directory is nearer the web than it is to its own
corpus's other three.

- **Not one of its 22 unopenable or pageless documents gets a page count out of `pdfinfo`**, so
  §29's broken claim is repeatable here without qualification. Fourteen of the seventeen pageless
  are one zip attachment, so the seventeen are five bug reports.
- **`MAX_FORM_DEPTH` is reached by seven, and all seven are cycles.** ADR 0271 established that
  over the crawl by lifting the bound to 256 and finding its four witnesses still reached it; the
  same experiment on these seven gives the same answer. Eleven documents, two corpora, no
  legitimate one — written into the constant's own comment with both denominators named, which is
  the `undenominated` sweep's rule applied before the sweep had to ask.
- **The 169 reports are all ones this project already owns**: 85 a glyph an embedded subset does
  not contain (closed by decision), 16 `Identity-H` with no embedded program (`doc/todo/21`), 58
  §11.4.7's non-isolated and knockout groups (`doc/todo/23`), 17 an undefined resource name, and
  181 single garbled tokens in damaged content streams.
- **The `slow` count is mostly the instrument again**, third chunk running: of four timed alone,
  two are 1.6 s and 0.5 s against 45.0 s and 43.2 s under eight-way load.
- **Two are genuinely slow and the first is the best witness `doc/todo/40` has ever had.**
  `MOZILLA-831621-14.pdf` opens in 2.1 ms, interprets in 414 ms into **3166 commands referencing
  3149 distinct clips** — one apiece — and then spends **41 seconds** rasterising them onto a
  1280 × 800 target, reporting nothing. That is exactly the chain arithmetic that item prices, on
  a page that is nothing else. `MOZILLA-892314-0.pdf` is the other shape — 162 commands, 83 clips,
  an 8646 × 3544 target, 32 s — and is a size rather than a structure.

## The second track

`--bin owed`'s reading list, oldest note first by blame on the note line: **§14.8, Tagged PDF**,
`partial` since 2026-08-13, and it is the `overstated` sweep's shape by hand — a parent naming
debts its own children have denied. It named two. §14.8.2.5's ordering was "a question for a
consumer that walks the tree rather than for the reader that builds it"; that row and all three of
its children are `implemented`, and a selection is taken in page content order because that is what
the shapes are in. And "most of §14.8.5's attributes — what a `/BBox` *says*" names as owed the one
thing that is read: `/BBox` **is** `Tree::bounds`, under Table 379's owner since ADR 0301 and under
Table 385's by the same priority, and it is on AT-SPI. Four of §14.8.5's eight children are
`implemented` and none is `reported`. The row now says what the family actually owes — the
*derived* rectangles rather than the stated ones — and stops counting its own rows, which is
§14.8.5's own rule one line down.

## What the next round should know

- `doc/todo/03` §31 is the cheapest thing here and it is a page four documents draw and we do not.
- `batch3` is fetched, extracted and walked. `batch2`, `batch4` and `batch5` are still owed, and
  the possible transfer ceiling above is what to test first.
- Two of 855's numbers about its own chunk were wrong in the same direction, and both were
  recoverable in minutes by re-running the walk. A count a round writes about a population it
  measured is worth re-deriving before it is built on.
