# 855 — The corpus that moved, and the page that does not finish

The primary item was `doc/todo/03`'s standing offer, handed on from 854: fetch SafeDocs' *Issue
Tracker* corpus — 31 GB in six archives that five sections of that file had been offering to
rounds since the five-hundred-and-forty-fourth — and take its first chunk.

## The corpus is not where the file says it is, and that took the first hour

`tools/safedocs corpora` knows one corpus and it is the crawl; the issue-tracker set was never
addressable by it. `doc/test-docs.md`, the owner's source material, says the corpora are
distributed "via **AWS Open Data** and the **Digital Corpora project**", and the second is
checkable: the bucket's own listing under `corpora/files/` has thirteen prefixes and not one of
them is it.

Where it actually was is the **Apache Tika regression server**,
`corpora.tika.apache.org/base/packaged/pdfs/pdfs_202011/`, which hosted it for the developers of
Tika, PDFBox and POI. That host is **NXDOMAIN** — Google's resolver answers `Status: 3` with
`apache.org`'s SOA in the authority section, which is the difference between "the server is down"
and "the name is gone". `LEGAL-696` says why: takedown requests, one of them described as
particularly threatening, HTTPS disabled, and the question closed on 2025-04-10.

**So the item was, for about twenty minutes, impossible.** What made it possible again is that the
Internet Archive holds all six tarballs and serves byte ranges of them — and, decisively, that
**Apache published a SHA-512 beside each one**, so a copy from a third party can be *checked*
rather than trusted. `batch6.tgz` came back at 1 303 317 582 bytes matching its published digest
exactly, and `batch1.tgz` matched its own. That is the whole reason this route is admissible under
principle 5's habits: the bytes are the publisher's, verified against the publisher's own number.

**Fetched and verified: `batch6` and `batch1`**, 3.27 GB compressed, 4.2 GB extracted, 4525
documents. **`batch2` was attempted eleven times and never matched** — the Archive answers a share
of requests with an nginx `504` page, and one attempt that did complete returned 4 201 315 360
bytes against the 4 201 352 112 the listing states, so its copy may be short as well as
rate-limited. `batch3`, `batch4` and `batch5` are untried. That is the remainder and `doc/todo/03`
§29 records it rather than glossing it.

**One trap sprung twice, in the same shape both times, and it is worth the sentence**: `curl -C -`
resumed onto a 504 page's bytes, so the digest could never converge and eight retries were spent
on a file that was wrong from its first 160 bytes. Each attempt starts from nothing now.

## The walk

One process per tracker directory, `doc/todo/03` §1's method and for its reason. A baseline for
these directories, never a ratchet:

| directory | documents | line |
|---|---|---|
| `batch6/cairo-gitlab` | 29 | 0 unopenable, 0 locked, 2 pageless, 2 incomplete, 0 slow |
| `batch6/evince` | 241 | 0 unopenable, 1 locked, 1 encrypted beyond us, 0 pageless, 14 incomplete, 0 slow |
| `batch6/poppler-gitlab` | 463 | 5 unopenable, 8 locked, 9 pageless, 54 incomplete, 3 slow |
| `batch1/PDFBOX`, eight shards | 3792 | 3 unopenable, 12 locked, 14 encrypted beyond us, 9 pageless, 275 incomplete, 2 slow |

**7.25% and 11.7% incomplete against the pdf.js corpus's 6.98% and the whole crawl's 1.735%.** A
corpus of bug attachments is four to seven times harder than the web, which is exactly what §1
says such a corpus is *for* — and the two figures are `CLAUDE.md`'s two denominators rather than
one number said twice.

## The head, which is one document filed by two people

`poppler-gitlab/poppler-978-0.pdf` and `PDFBOX/PDFBOX-3688-0.pdf` are **byte-identical** — 2 051 350
bytes, SHA-256 `3030bb601a45da4c426d9a77b166a77991ea230c9c0f478508bf516ffcda7618`. Two trackers,
two reporters, one file. It **never finishes**, and every existing bound passes it: it opens in
1.6 ms, interprets in 2.5 s into 298 379 commands, and reports **nothing**. Ten sampled stacks put
nine of ten in one call — `draw_group` → `PixmapMut::draw_pixmap`. Page one states **73 047
transparency groups**, each isolated, unclipped, soft-masked and holding nine images, on a
1701 × 2409 target: 4.1 M pixels blitted apiece, some 300 billion for the page, measured at 115
groups a second, which is about 640 s and matches both surveys' `slow` line at 616 s and 602 s.

**The obvious fix was built and then disproved, which is the part worth keeping.** `render-cpu`
already has `marked_rows` — the rows a command list's own marks can reach — and `build_soft_mask`
already uses it; narrowing a *group's* band by it is three lines and is exact, because outside its
own marks an isolated group's buffer is the transparency it was allocated as and all three
operators a group is blitted under leave a destination alone under a transparent source. It buys
nothing here: every one of those groups' nine images spans the page's full height, so `marked`
comes back as all 2409 rows, 4497 samples out of 4546. And it buys nothing on the two documents
`doc/todo/40` prices for this exact shape either — `0423548.pdf` 2.35 → 2.37 s, `3990833.pdf`
2.78 → 2.79 s, three samples each. **Reverted**, on `CLAUDE.md`'s own rule that an optimisation is
justified by a benchmark; had it been kept it would have been a clever line nobody could defend.

What the page needs is a **bound**, and this round deliberately did not invent one: the measure is
cumulative group-blit pixels rather than a group count, and the constant has to be sized against a
population the way `MASK_BUDGET` was. `doc/todo/03` §29 says so with the census named.

**Two of the three "slow" documents in that survey are not slow** — 260 ms and 68 ms alone against
602 s and 601 s under 24-way load beside the one above. §1's own warning about that count, met
again.

## The defect that was fixed

A different report, and it misstates the file. `evince-1360-1.pdf` printed *no `/Font` resource
named `/f-0-0`* six times for a page whose `/Resources /Font` names all six of its fonts — what the
bug report's reduction removed is objects 6 to 11. §7.8.3 is a resource the file never defines;
§7.3.10 is a reference to an object it does not carry, which "shall be treated as a reference to
the null object". `Font` was the last resource category folding the two, `XObject` having told
them apart since ADR 0255. ADR 0779, and the split is measured: **0 of the 974's 2 reports move,
12 of 12 in the cairo and evince trackers, 10 of 80 in PDFBOX's** — the counted population keeps
its wording and only what was never in it changes.

## The second track

`--bin owed`'s reading list, oldest note first by blame: **§11.3.4**, `partial` since
2026-08-11 with a note that names no debt at all. The clause's own list is six spaces that "shall
be supported as blending colour spaces", and `space_departure` admits three without a report;
`DeviceCMYK` is the fourth and is composited by §11.4.7's two-raster construction. What is left is
the **one-component** row and **`ICCBased` 'CMYK'** — and neither is a formality, because §11.6.6
makes a group's space the one its elements are converted *to*, so a red mark inside a grey group
composites as grey and this tree keeps it red. Written into the row, which takes it off the sweep
for the right reason.

## What the next round should know

- **A claim that has held for six populations breaks here.** "Nothing failed to open for a reason
  that is this tree's" cannot be said of this corpus: eight of the sixteen documents these four
  directories could not open get a page count out of `pdfinfo`. Two were read by hand and neither
  is ours — `poppler-303-0.pdf` states `/Kids 3 0 R` and has no object 3, and `poppler-732-0.pdf`
  has no `%PDF-` header anywhere while poppler answers 33 pages for it. **Six are unread**, and
  they are the cheapest thing this chunk leaves.
- `batch3`, `batch4` and `batch5` are untried and `batch2` needs a different day or a different
  mirror.
- The group bound above wants its census before its constant.
