# What the corpus and the oracle still name

Status: **standing** — `tools/state.sh` prints the counts; the populations live here.
Read by: whoever is taking a page off the ambiguous ranking, reading a contradicted verdict, or
deciding what an `ambiguous` means. `doc/todo/00-ambiguous-bucket.md` is the task and the method;
this is the population and what it has produced.

`doc/HANDOVER.md`'s reader table points a round judging pages against other renderers here.

**Read the oracle's 45% ambiguous with care.** 370 of those pages are two long books of dense
text at book size, where `Interpretation::glyphs` earns the page the *text* tolerance — 0.90
structural similarity, measured over 153 reference-against-reference pairs because the references
disagree with each other at worst-tile 26 to 28 on text. **This file said for many sessions that
those books are "set in fonts nobody embedded, so each renderer substitutes differently", and
`pdffonts` says otherwise**: `freeculture.pdf`'s four fonts are all embedded and nothing
substitutes on any of its pages (the two-hundred-and-twenty-ninth session, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`).
That row means "reported nothing", not "drew it right" — and since the hundred-and-seventy-fifth session **emptying it is a task rather
than a caveat**: §3a.

**Both moving numbers move in both directions on purpose.** Contradicted pages: 174 → 65 over
sessions 6 to 61, steady at 65 until the hundred-and-forty-eighth took it to 70, the
hundred-and-fifty-sixth to **72** and the two-hundred-and-fifth back to **68** — the last two were `noembed-eucjp.pdf` and `noembed-sjis.pdf`,
recorded as drawing あいうえお "in a face the references do not have" — **and they were drawing
nothing at all**, which the hundred-and-eighty-second session found by making the silence loud
(ADR 0152). Both report now, and two widget-border pages left in the two-hundred-and-fifth session (ADR 0165), so the count was **69** until the four-hundred-and-fifth, whose one width defect took it to **68**. Five of the earlier ones were net,
argued and written down in `CONTRADICTED_SUBSTITUTED_FONT`: the standard 14 are compiled in now,
so we stopped reading the same URW faces off this machine's disk that the three C references
read, and the oracle noticed within one run (ADR 0133). Corpus documents drawing incompletely: 291 → 89 over
sessions 6 to 122, then 91 in the hundred-and-twenty-seventh, where two documents that had been
drawing the wrong font in silence started saying so, and **76** in the hundred-and-fifty-sixth.

### 2. The corpus is four populations now, and only one of them is ratcheted

**Until the four-hundred-and-twenty-second session this file described one corpus**, the 974
pdf.js documents, and `CLAUDE.md`'s "Two questions, two denominators" says robustness is measured
against *the world*. Three more arrived as submodules under `doc/corpora/` and a fourth as a
fetcher, and none of them is in `doc/todo/02` §2's default sequence — deliberately, because that
sequence is 268 s and a new corpus earns a place rather than taking one. The instrument is
`tools/safedocs survey --dir <path>`, which asks the same five questions `tests/corpus.rs` asks
and *ratchets none of them*.

**The first run is the baseline, and this is it** (ADR 0258):

| population | documents | complete | reported | licence |
|---|---|---|---|---|
| `doc/pdf.js/test/pdfs` (the gate) | 974 | — | **65 incomplete** | — |
| `doc/corpora/pdf20examples` | 7 | **7** | 0 | CC BY-SA 4.0 |
| `doc/corpora/pdf-differences` | 37 | 30 → **29** | 7 → **8** | CC BY 4.0 (the PDFs; the repository's code is Apache-2.0) |
| `doc/corpora/pdfbox` (`.../test/resources/input`) | 64 | 63 → 62 | 1 → **2** | Apache-2.0 |
| SafeDocs `CC-MAIN-2021-31`, archive `0000`, first 24 | 24 | 22 | 2 | crawled web, no grant — never committed |
| SafeDocs `CC-MAIN-2021-31`, archive `3500`, first 24 | 24 | 22 | 2 | crawled web, no grant — never committed |
| **SafeDocs `CC-MAIN-2021-31`, 79 archives `50 + 100k`, first 24 of each** (session 425) | **1896** | **1802** | **86** → 85 | crawled web, no grant — never committed |
| **SafeDocs `CC-MAIN-2021-31`, 4 whole archives `0100 + 2000k`** (session 430) | **4000** | **3917** → 3923 | **70** → 64 | crawled web, no grant — never committed |
| **SafeDocs `CC-MAIN-2021-31`, all 145 archives** (session 433) | **65 944** | **64 507** | **1144** → 1138 → 905 → 851 → **824** | crawled web, no grant — never committed |
| **`openpreserve/format-corpus`, five directories** (session 467) | **267** | 239 → 237 | **21** → 22 → 23 | **decided, and three of the five are in the tree** — §2b |

**The bottom row is not a sample of anything and is read in §2b** — 267 hand-made and
hand-collected files, examined in the four-hundred-and-sixty-seventh session; its moved figures are
ADRs 0302's and 0303's, and both rises are new reports rather than regressions. **Three of its five
directories are a submodule as of the four-hundred-and-seventieth** (ADR 0305), which is where §2c's
question went; the survey line for each of the three, taken on the day they were pinned, is in §2b.

**The row above it is the whole SafeDocs population, and the first survey to find a hang** (ADR 0269). **Its reported column is the only figure in this table that four later rounds moved**, each re-surveying all 65 944 whole: 1138 after session 434's substitution rule, 905 after the four-hundred-and-thirty-sixth made the press the document's own — 233 documents becoming complete — 851 after the four-hundred-and-fortieth took a soft mask's group off §11.4.7's route, and **824** after the four-hundred-and-forty-first closed §11.3.5.3's row (ADRs 0270, 0272, 0276, 0277). The four-hundred-and-forty-fifth did **not** re-run it — 93 GB is not a closing round's instrument — so 824 is session 441's printed number and is labelled as one rather than re-claimed.
**65 944 documents in 1139.3 s: 173 unopenable, 45 locked, 23 encrypted beyond us, 52 pageless,
1144 incomplete, 2 slow**, with 51 272 codes reaching no glyph in silence over 635 documents — a
baseline for this population, never a ratchet. **The four-hundred-and-thirty-fourth session read
that last number and moved two of them** (ADR 0270): the 51 272 are 28 837 codes over 359
documents that were never a mark missed — a glyph the font program contains and describes with no
contours — and 22 435 over 277 that were, of which one mechanism held 4912; with it fixed the same
pass prints **1138 incomplete and 780 codes over 236 documents**, and the six reports that left are
named in the ADR. It was run as **one process per archive**, because
`render-cpu` rasterises under `panic = "abort"` and one document's abort would otherwise take every
other verdict in the process with it; five of the 145 archives died on the first pass and both of
this round's defects are those five. Four things it says:

- **1.735% is the web's rate**, against session 430's 1.75% of 4000 and session 425's 4.54% of 1896.
  The first moved because sessions 426 and 427 built §11.4.7's conversion; the last two differ by
  0.015 points over a sixteen-fold increase in sample size. **The 974 are at 65, which is 6.7%** —
  four times the web — and that is what a corpus assembled from bug reports is for. (The web's own
  1144 fell to 824 over sessions 434, 440 and 441, so the ratio holds while both numbers move; this
  bullet's corpus figure said 68 and 7.0% until the four-hundred-and-forty-fifth read the gate.)
- **The largest population is still a group's blending colour space**, 398 documents, 0.60%; and it
  splits into `doc/todo/23`'s own rows with numbers at last — 151 documents name the press their
  `DeviceCMYK` is, 106 state a page group whose four components are not `/DeviceCMYK`, 78 have a
  group inside the page compositing in a different space and 7 state Table 57's `/BG` or `/UCR`.
- **A budget stopped interpretation on 84 of 65 944, 0.127%**, stable across three samples, and
  neither of the two slow documents is one of them.
- **Nothing failed to open for a reason that is this tree's, for the third sample running**, and the
  five whose cross-reference table is unusable were opened by hand: three have had their `<<` and
  `>>` turned into `&gt;` in transit and two are truncated to about a hundred bytes.

**The row above it is the first look at the web with §11.4.7's conversion built** (ADR 0266), and what
it says is about the *residue*. **4000 documents in 53.3 s: 6 unopenable, 3 locked, 2 encrypted
beyond us, 2 pageless, 70 incomplete, 0 slow**, with 1161 codes reaching no glyph in silence over 33
documents; **64 incomplete after that session's own two fixes**. The rate is what moved — 86 of 1896
was 4.5% and 70 of 4000 is **1.75%** — because §11.4.7's population fell 67 → 24 on sessions 426 and
427's work and nothing else changed between the two samples. Three readings, and the ranking is in
`doc/todo/03`:

- **Nothing failed to open for a reason that is this tree's, for the second sample running.** All
  six unopenable documents have no `%PDF-` header in their first kilobyte; the two pageless ones and
  two of the three undecodable content streams are the crawl truncation ADR 0261 named.
- **The largest *named* residue is a four-component `ICCBased` page group** — 14 of 4000, every one
  of them checked with `examples/group_space_census` — where `doc/todo/23` had it at one witness.
- **Two defects were fixed from it**: a three-component JPEG carrying an Adobe APP14 marker, which
  lost 21 images over four documents, and a `/Contents` part the file states is empty, which was
  reported as drawing the page had lost. Neither moves the 974, because no document of the 974 does
  either thing.

**The row above it is the first sample large enough to rank a population, and it does** (ADR 0261).
1896 documents in 42.1 s: 4 unopenable, 1 locked, 0 encrypted beyond us, 3 pageless, 86 incomplete,
0 slow, with 862 codes reaching no glyph in silence over 12 documents. Two readings of it:

- **Nothing failed to open for a reason that is this tree's.** All seven unusable documents were
  opened by hand and are crawl artefacts — four HTML pages saved under a `.pdf` name, three PDFs
  the origin server truncated at about a kilobyte.
- **67 of the 86 reports are §11.4.7's page-group blending space**, `doc/todo/23`'s standing item.
  That is **3.5% of the web against 0.7% of the 974**, which makes it the largest correctness gap
  this tree has against real files by a factor of six over everything else together. 7 more are
  `doc/todo/21` §3, 4 are §11.4.4, and the remaining 8 are singletons. **The count fell 86 → 85 in
  the same session**, on §7.10.4's k: a real document's four shadings were refused whole because a
  bound documented as counting a function's components was being applied to its subfunctions, and
  the same clause's nesting was a stack overflow (`crates/pdf-model/tests/hostile_functions.rs`).

**And the archives are hash buckets, not crawl neighbourhoods.** The corpus is the whole crawl
sorted by SHA-256 and cut into 7933 equal pieces — over 1944 members a file's number and its digest
agree to 2.6 × 10⁻⁴, which is the order statistics' own fluctuation — so any window anywhere is an
unbiased sample of all 7 932 878 and a round may go deep without paying for spread. `doc/todo/03`
carries the rule that replaced the stratification one.

**Two of the four are partial, sparse submodules and `.gitmodules` cannot say so**, so the recipes
live here. Each is the clone a fresh checkout wants *instead of* `git submodule update --init`,
which also works and takes the whole repository:

```sh
git clone --depth 1 --filter=blob:none --sparse https://github.com/apache/pdfbox.git \
          doc/corpora/pdfbox
git -C doc/corpora/pdfbox sparse-checkout set pdfbox/src/test/resources/input

git clone --depth 1 --filter=blob:none --sparse \
          https://github.com/openpreserve/format-corpus.git doc/corpora/format-corpus
git -C doc/corpora/format-corpus sparse-checkout set \
          pdf-handbuilt-test-corpus pdfCabinetOfHorrors govdocs1-error-pdfs
```

1.9 s and 2.6 s for the first, against 118 MB for the plain clone; 1.6 s and 3.1 s for the second,
which is 73 MB checked out and about 58 MB of pack against a repository holding every file format
the Open Preservation Foundation collected. **Neither is needed by any gate**, which is what makes
them safe to leave uninitialised: the two tests that name a path inside one print a line saying it
is not checked out and pass.

**What the 132 new documents said, and what it is worth.** Five of `pdf-differences`' seven
reports are the point of the file — its `UnknownFilter` set encodes one stream apiece with a fake
`/XXXDecode` and its own `README.md` says which of them a reader should survive — and `pdfbox`'s
one is `MAX_FORM_DEPTH`. The two SafeDocs reports are §11.4.7's `/DeviceCMYK` page group and
§11.4.4's non-isolated group, both populations `doc/todo/23` already names, which is why nothing
was promoted from that fetch. **The one thing that was not already known is the finding**, and it
was a silence rather than a report: `UnknownFilter-PageContentStream.pdf` came back *complete*
with zero commands, because its content stream object's dictionary ends with one `>` where §7.3.7
requires two, and §7.3.10 then makes the reference null and §7.3.9 makes null an absent entry. A
page whose producer named a content stream and got a blank one is not a page whose producer stated
none; `ContentIssue::Unreachable` is the difference, and **the pdf.js corpus's own count did not
move by it**, which is the sharpest available statement of why a second corpus was worth a
session.

**What the SafeDocs fetcher refuses, and what it does not**, because a population nobody can hold
is a different kind of instrument: the archive is addressed a member at a time and never as an
object, nothing transfers without `--download`, and a plan over 32 MiB is refused in bytes **and
in the `--budget-mb` that would admit it**. The budget has no ceiling and `--all` takes every
member of an archive, so a whole 1.6 GiB archive on an unmetered connection is one deliberate
command — the bound is on *accident*, not on the person. Every member is checked against the
CRC-32 its own archive records. `tools/safedocs/src/lib.rs` and ADR 0258.

### 2a. `pdfbox` carries somebody else's answer as well as their documents, and it is a gate

**Since the four-hundred-and-twenty-third session** (ADR 0259). `doc/corpora/pdfbox`'s
`input/` directory holds `*.pdf.txt` and `*.pdf-sorted.txt` beside **40** of its 64 PDFs —
Apache PDFBox's own `PDFTextStripper` output, checked in as a fixture. That is a different
instrument from `pdftotext`, which runs at gate time and answers whatever this machine's
poppler answers today: a frozen opinion cannot drift under this tree, and it was written by
people who read §9.10.2 independently.

`text_extraction.rs::the_text_we_draw_agrees_with_pdfboxs_frozen_extraction`, sharing that
file's `fold`, `reference_words` and `without_spaces` so that a difference between the two
references is a difference about the documents rather than about the comparison. Whole
documents rather than page one, because `PDFTextStripper` walks every page and `cweb.pdf` has
28. Both of PDFBox's texts are read and only the stream-ordered one gates; the `-sorted`
figure is printed beside it, because where the two agree, *reading order* is not what a
shortfall is about.

**It is in `doc/todo/02` §2's sequence with no new line and at 0.4 s.** Line 28 already runs
every ignored test in that binary, and this one's reference is a file rather than a process —
the pdf.js gate spends 30 s of its 31 waiting for 974 `pdftotext` invocations.

The first run, which is the baseline: **40 documents, 99.8% (14254/14281 words) against both
of PDFBox's orders, 5 below the 0.90 floor.** Every one of the five was read before anything
was ratcheted, and **three of them were one defect** — §9.10.2 excludes an `Identity-H`
composite font from its third method *by name*, so a `/ToUnicode` that answers for some codes
or none leaves every method failed, and the permission the clause then grants was being
declined. Fixed; the pdf.js gate went **23987 → 24003** of `pdftotext`'s 24187 words with **25
named documents below the floor → 23**, and one of the two that left had been recorded as
undiagnosed for 357 sessions. The four that remain are named in `PDFBOX_BELOW_FLOOR` with the
reading beside them: two are right-to-left text in painting order and in presentation forms
(§14.8.2.5.1, and neither file writes §14.8.2.5.3's `/ReversedChars` — measured over all 108
new documents), and two are the one place this tree and PDFBox make different **choices** under
the same permission, where PDFBox reads a two-byte code as a Unicode value and this tree will
not.

**A raster was pointed at the same 64 in the five-hundred-and-fifty-fourth**, which nobody had
done: page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, ranked by our ink minus the
lightest live reference's. The ranking separated nothing — the whole negative tail is −0.410 and
shallower and the three deepest are one page four times — and the finding came out of the *size*
column beside it, on a page every renderer draws blank at two different sizes (ADR 0389,
`doc/todo/03` §13).

### 2b. `openpreserve/format-corpus` — three directories of it are the fourth submodule

**Taken in the four-hundred-and-seventieth session** (ADR 0305), on the project owner's rule that a
corpus is added unless its licence clearly forbids it — a submodule being a pin rather than a copy,
so nothing is republished by it. `doc/corpora/format-corpus` is pinned at `366f068c` and
sparse-checked out to **`pdf-handbuilt-test-corpus`, `pdfCabinetOfHorrors` and
`govdocs1-error-pdfs`**; §2 above has the clone recipe and `doc/third-party-data.md` has each
directory's own terms, quoted, and the two that were left with the reasons. The whole five-directory
population is still fetchable into `corpus-cache/`, which is `.gitignore`d, and that is what the 267
in the table above were surveyed from. `/pdf/`, which `doc/test-docs.md` used to name, does not
exist.

**The survey line of each pinned directory, on the day it was pinned** — a baseline for that
population and never a ratchet, the way every row of §2's table is:

- `pdf-handbuilt-test-corpus`: *89 documents in 0.1 s: 1 unopenable, 0 locked, 0 encrypted beyond
  us, 3 pageless, 13 incomplete, 0 slow*, 0 codes reaching no glyph in silence.
- `pdfCabinetOfHorrors`: *24 documents in 1.2 s: 0 unopenable, 1 locked, 0 encrypted beyond us,
  0 pageless, 2 incomplete, 0 slow*.
- `govdocs1-error-pdfs`: *54 documents in 3.7 s: 0 unopenable, 0 locked, 0 encrypted beyond us,
  0 pageless, 6 incomplete, 0 slow*, with 52 codes over one document reaching a glyph the font
  draws blank, which is not a mark missed (ADR 0270).

**`pdf-handbuilt-test-corpus` is an instrument rather than a sample**, and it is the reason this
chunk was worth a round. Its 89 files each carry **one** deliberate structural defect against
ISO 32000-1's requirements — a header without a version, a trailer without a `/Root`, an `xref`
whose offsets are wrong, a `Tf` with its keyword deleted — and every one of them draws the same
*Hello PDF-world!*. So a file that comes back blank is a file whose defect cost a mark, which turns
a survey into an assertion:

```sh
for f in doc/corpora/format-corpus/pdf-handbuilt-test-corpus/*.pdf; do
  cargo run --release -p pdf-model --example render_at -- "$f" 1 1 /tmp/p.png >/dev/null 2>&1 &&
  magick /tmp/p.png -alpha off -colorspace Gray -format "%[fx:255*(1-mean)]\n" info:
done
```

The intact page reads **0.807367** and most of the files reproduce it exactly. **Fourteen read 0**,
and nine of those said why; the whole finding was the other **five, blank in silence**. Two of the
five are right — Table 31 makes a page stating no `/Contents` empty, and a file whose text-showing
operator was deleted has nothing to show. **All three of the rest have now been taken**: ADR 0302's,
a show operator that disappeared because its operands were read from the wrong end; ADR 0303's, a
`Tf` whose size operand is a lone `.` that this lexer read as a number; and ADR 0305's, a page tree
node with no `/Kids` that was drawn as a page, whose file now reads **0.807367** like the intact
one. **Thirteen read 0 today and not one of them is silent about it** — eleven report, two are the
blank the standard asks for — which is the strongest statement this instrument can make and is what
made 0.1 MB of files worth three rounds against 93 GB of crawl.

**And that is the strongest statement about *blankness*, which is narrower than it reads.** The
five-hundred-and-fifty-fourth session arrived at
`T02-03_008_page-object-mediabox-missing.pdf` and `T02-03_009_page-object-mediabox-not-rectangle.pdf`
from `pdfbox`, not from here: both draw their *Hello PDF-world!* at 0.779 rather than 0.807367 —
close enough to read as glyph weight and produced by a page **596 × 842 where three references say
612 × 792**, because neither file states a usable `/MediaBox` and this tree substituted one in
silence (ADR 0389). A file that draws the right ink on the wrong sheet passes the assertion above
and always would have. **An instrument is spent for the predicate it was asked**, and this one's
predicate is a zero.

**The other four directories are ordinary populations** and produced nothing new:
`pdfCabinetOfHorrors` (24, archival horrors: encryption, embedded video, a corrupt byte) reports a
JPEG whose `/Height` was altered and an `/Im0` that is not a stream, both of which are the file's
stated defect — **and the first of the two was a defect of this tree as well, found in the
five-hundred-and-fifth by the ink ranking of `doc/todo/03` §8**: the altered `/Height` cost the
whole photograph until §7.4.8 was read, and the file now draws pixel-identical to the intact one
while still reporting the contradiction (ADR 0340); `govdocs1-error-pdfs` (54, `.gov` crawl files that broke somebody else's software)
reports four unparsable CFF programs, a truncated `head` table and an undecodable `/Contents`;
`jhove-errors` (99, real published papers that JHOVE calls invalid) reports one `/Font` a page
names and does not define and one `/ExtGState` likewise; `fully-featured-pdf` (1) is complete.
**Nothing failed to open for a reason that is this tree's**, for the fourth population running.

**"Ordinary" was a statement about those two reports and not about the population, and the
five-hundred-and-forty-fourth session found the difference** by doing to `jhove-errors` what
nobody had done to a directory outside the sparse checkout: ranking it against three references.
It also held a **pageless** document — a 21-page paper whose correct `startxref` sits eight
megabytes from the end of the file, behind a truncated copy of itself, which this tree drew no
page of at all (ADR 0379). Its line reads *0 pageless* now. And two of its files are the only
documents on this disk that render here and in **no** reference: `PDF-HUL-29`'s pair, where
poppler, mupdf and ghostscript each refuse a `/Kids` entry that is not an indirect reference and
this tree draws the page.

### 2c. The licence question `openpreserve/format-corpus` raised, read and answered

**Read in the four-hundred-and-sixty-seventh session and answered by the project owner before the
four-hundred-and-seventieth.** The repository's root `README.md` says "All items are CC0 licenced
unless otherwise stated", and
`doc/todo/03` has said since the four-hundred-and-twenty-second that the work owed is to open the
per-directory sidecars and find out whether any of them states otherwise. They do, and the result
splits the five directories three ways:

| directory | what its own files state |
|---|---|
| `pdfCabinetOfHorrors` | **CC0, explicitly**: its `readme.md` ends "All files in this folder: Creative Commons CC0: Public Domain Dedication." |
| `pdf-handbuilt-test-corpus` | **nothing**, so the root default would apply — but its `README.md` points at a deposited research artefact (DOI 10.22000/53, ipres2017) whose own terms are not restated |
| `govdocs1-error-pdfs` | **otherwise stated**: "All PDF files in this folder and subfolders are copied from Govdocs1", quoting Govdocs1's own "may be (to the best of our knowledge) freely redistributed" — a statement of belief and a citation request, not a grant |
| `fully-featured-pdf` | **nothing**, and the file embeds third-party media (an MP3, a QuickTime movie, a U3D model) the README does not license |
| `jhove-errors` | **no sidecar at all**, and its 99 files are published journal articles and theses — Springer, Wiley, university repositories. A third party cannot dedicate those to the public domain, so the root default is unreliable here rather than merely silent |

**The owner did not answer the question; they replaced the rule that produced it**, and the words
are in ADR 0305 with the argument. The short of it: add a corpus unless its licence *clearly*
forbids, because a submodule is a pin and pinning republishes nothing — mention them all as a
courtesy anyway. Under that rule three of the five are in the tree and the table above is what each
was read against; `doc/third-party-data.md` carries the attributions and says plainly which
directories state nothing rather than implying terms they do not carry.

**The two that were left were left on size and on value, not on their terms**, and saying so is the
point: `jhove-errors` is 275 MB of published papers under no grant anybody could make — an absent
grant rather than a prohibition — whose survey produced two ordinary reports; `fully-featured-pdf`
is one already-complete document whose distinguishing half is Clause 13, which `CLAUDE.md` excludes.
Neither is closed to a later round that wants it.

**What has not changed**: no file from any of the five is a candidate for committing outright, and
`doc/third-party-data.md`'s rule binds here exactly as it binds SafeDocs. A test names a path inside
the submodule and skips where it is absent — `crates/pdf-model/tests/page_tree_nodes.rs` and
`contents_entry.rs` — so no gate in `doc/todo/02` §2 depends on the checkout.

### 2d. `pdf-association/pdf-differences` is not an oracle population, and that is a decision

**Taken in the five-hundred-and-fifty-eighth session** (ADR 0393), which is the round that ranked
it — the last of the five populations on this disk to be put in front of a reference.

`doc/todo/03` §4 had held this corpus back since the four-hundred-and-twenty-second on a worry
about the verdict vocabulary: files chosen for disagreement would come out `ambiguous` almost
everywhere, and a page where this tree differs from three references *by the standard's own
permission* is not a contradiction. **The worry was about a corpus that does not exist.** The name
means implementations differ; sixteen of the eighteen test cases quote a normative sentence of ISO
32000-2 and then publish the correct picture, and the repository's own README makes that a
convention — "Correct renderings are always the _last_ image in the MarkDown". Exactly two
differences in the whole set are the standard's own permission, §8.4.3.4's zero-length dash at a
zero-length subpath segment and §9.5 NOTE 5's substitution, and both are stated as such in the
standard rather than in the corpus.

Three consequences, and the first is the one that binds this file:

- **It may not go through the oracle's vote, and the reason is stronger than principle 5's usual
  one.** On this population the references are the *subject under test* — the files exist because
  implementations split on them — so a vote reads the answer off the very programs the corpus was
  assembled to catch out. The round's own numbers say what that would have cost: at least one
  reference is wrong against the clause on six of the eighteen cases, and on `Type3Test.pdf` **two
  of the three are**, so the majority reading is the wrong one and a vote would have ratified it.
- **`pdfref::Outcome` gains no verdict.** Every outcome it has is a function of the rasters;
  "the standard permits this difference" is a function of a clause, and a term the instrument
  cannot compute becomes the bucket every page nobody wants to explain goes into. A permitted
  difference is `ambiguous`, or `contradicted` where the references happen to agree on one of the
  permitted answers — **and it stays `contradicted`**, because that verdict is a true statement
  about the evidence and hiding it would lose the one fact a later round needs. What converts it
  from an accusation into a documented choice is a **named group quoting the permission**, which is
  what `CONTRADICTED_SUBSTITUTED_FONT` has always been. ADR 0393 §2 has the argument.
- **What it is for instead**: a reading list of eighteen clauses with hand-built witnesses, and a
  per-case gate wherever the clause supplies the expected value with no reference in it — the same
  shape as §2b's one-line ink assertion. `IndexedColor`'s "both rows shall match exactly",
  `Inline-Image-Abbreviations`' "the same image in all eight locations" and §8.4.3.5's
  `w/(2·sin(φ/2))` are three that are ready to be written.

### 3. What the corpus still names

**The oracle's 68 contradicted pages, 66 of them on documents we call complete**, grouped and
ratcheted in both directions in `oracle.rs`, where each group carries its own diagnosis and its
measurement. **The 66 counted off the groups themselves in the four-hundred-and-fifth session**,
because this paragraph's own list summed to 72 and had said "4 page rounding" for a group of 2 and
"21 substituted fonts" for a list of 18: 2 page rounding, 2 our own anti-aliasing at a shape's
edge, **21 glyph edges** whose ink matches the consensus to a fraction of a level, 7 a shared
JBIG2 decoder, 1 a visibility expression the two agreeing references share a *gap* about, 3 a link
border, 1 a sub-pixel image, 1 a `CalRGB` alternate, 1 an eight-bit mask value, **5 a `DeviceCMYK`
conversion**, 2 a reference that drew nothing, 1 a reference glyph width, 1 a negative line width,
**17 substituted fonts**, **1 a tight consensus**, **0 unexplained**. The other 2 were on documents
this tree already reported (`issue5751.pdf`, `knockout_blend_multiply.pdf`) and held by the
incomplete list rather than by a group — **and the second of those left in the
four-hundred-and-seventy-second**, which is what a page held by the incomplete list is for: it
contradicted all three references at mean 23.96 because §11.4.6's `/K` was substituting §11.4.5's
transparent backdrop for a group whose knockout rule could show nothing, and it agrees with them
now (ADR 0307). The figures in this paragraph are the four-hundred-and-fifth session's and
`tools/state.sh oracle` is what says today's.

**A count beside a list is not the list**, which is `doc/todo/02` §6's rule arriving one directory
over: the numbers above are now what `oracle.rs`'s arrays hold, and the way to keep them so is to
count them rather than to adjust them.

**The unexplained list is empty**, from 14 four sessions ago and from 42 at the start, and no
session that emptied it opened a debugger — the method is in
[todo 00](todo/00-ambiguous-bucket.md), which is the same method the ambiguous work uses. The
last two went to the two-hundred-and-forty-second and -third, both on the two-ladder closed form:

- `freeculture.pdf` page 313 → `CONTRADICTED_GLYPH_EDGES`. Ours at 8× is **6.0729** against a
  limit of 6.0658 and 6.0819, so the marks are right and the difference is 0.16 of 255 of glyph
  coverage at the page's own scale.
- `issue7891_bc1.pdf` → `CONTRADICTED_TIGHT_CONSENSUS`, the new name for what trap 12 describes.
  The two ladders agree to **0.0014 of 255** — the tightest limit in this file — and **ours at
  the page's own scale is 0.004 from it, the nearest of all five**, while `poppler` and `mupdf`
  are both 0.09 under. They vote because the bound is twice *their* spread, and they agree to
  0.009.

**Every printed metric on both pages is inside the class bound.** A verdict of contradicted can
be a statement about the consensus pair rather than about the page, and both of the last two were
that — which is the argument for the closed form: it is the one number derived from no reference
at all.

### 3b. The contradicted list has a ranking, and its lines report the right renderer

**Both arrived in the four-hundred-and-sixth session and neither moved a verdict** (ADR 0242). The
gate's per-page line for a contradicted page was measured against whichever reference had the largest
tile, which need not be one the verdict rests on, and printed beside a bound derived from the pair
that does; and it printed **three** of `Tolerance::accepts`' **four** bounds, the differing fraction
appearing as our number with nothing to compare it to.

Both had consequences legible in the gate's own output before a line was changed:

- **Thirty of the sixty-eight contradicted pages printed a line on which every visible number was
  inside the printed bound.** `issue7580.pdf` is the plainest — mean 2.93 of 5.00, worst tile 7.10 of
  40.00, ssim 0.9734 of 0.9000 — and `differing 6.15%` against a 5.00% nothing printed.
- **`smask_luminosity_oob_transfer.pdf` printed 27.02 against a bound of 1.11.** The 27.02 is
  `poppler`, which is not in the consensus and sits 34 to 36 of 255 from all four other renderers on
  that page while they sit within 1.7 of each other; our distance from the pair that decides it is
  **1.25**. `CONTRADICTED_MASK_QUANTISATION` had to state its own numbers because the gate's were
  somebody else's.

**Forty-three of the sixty-eight now report a different comparison, none of them passes every bound,
and thirty-eight fail on exactly one — the differing fraction.** The largest thing that moved is a
diagnosis: `CONTRADICTED_GLYPH_EDGES` had opened since the seventy-fifth session with "[e]ach fails
**only** on mean absolute difference — 5.4 to 6.4 against a bound of 5.00", and every number in that
sentence was `ghostscript`'s on 21 pages that all read "poppler and mupdf agree". Against the pair
that decides them the means are 1.01 to 2.57 of 5.00 and **all 21 fail on the differing fraction and
nothing else**. The group's diagnosis is *strengthened* by the correction, because a count of channels
that moved at all is precisely what a sub-pixel phase shift produces and an average is precisely what
it does not.

**And the list is ranked, for the first time in four hundred sessions.** `rank_the_contradicted`
prints the ten pages furthest from their *nearest* reference, the instrument the ambiguous bucket has
had since the hundred-and-seventy-sixth. In bounds the head is the JBIG2 pages; taken in levels of
255 by hand it is `bug847420.pdf` at 8.65 from the nearest of four renderers that agree among
themselves to 4.64, twice as far as anything on the list that is not a link border. That page was
re-derived in the same session and is `CONTRADICTED_SUBSTITUTED_FONT`'s name for once: `/Widths` are
honoured to **1420 device columns against both references' 1419** at 8×, and what differs is the face,
3.6% lighter at every scale. §9.5 NOTE 5 is the clause that leaves it open — "some details of font
naming, font substitution, and glyph selection are implementation-dependent" — and the group had
argued from it for four hundred sessions without citing it.

**And the group is twelve rather than seventeen since the four-hundred-and-thirty-first session**,
which measured the five pages nobody had ever opened. `CONTRADICTED_SUBSTITUTED_FONT` admits a page
on what it *carries* — the page names a font nobody embedded — which its own first paragraph calls
the weakest rule in the file, and six of its seventeen had been re-derived one at a time over four
hundred sessions while five had come in together and stayed. All five name `/Times-Roman`, and on
all five the substitution is invisible: at 8× the ink's bounding box is **1233 × 143 at (84, 133) in
ours, `poppler`'s and `mupdf`'s alike** on `issue8088.pdf`, identical over a 1600-column raster, and
one column in 1440 apart on `bad-PageLabels.pdf` and `franz_2.pdf`. Equal width is §9.2.4's advances;
equal *height* is the cap height, which nothing in the standard states. Each fails the differing
fraction and nothing else, at 5.01% to 5.68% of 5.00%, so they are `CONTRADICTED_GLYPH_EDGES` and
moved there — which takes that group to **26**.

**What is left of this group's sans half is one number.** The compiled-in Helvetica is Liberation
Sans and the references resolve `NimbusSans`; drawn straight from the two files, the capital `I` is
**0.687500 em** against **0.729167 em**, in the regular and the bold alike, and the corpus rasters
reproduce both exactly — `issue6108.pdf` at 12 pt draws 66 device rows against 70, `issue7580.pdf` at
18 pt draws 99 against 105. 5.7% shorter capitals is 1.0% to 7.7% of the page's ink on the six pages
that name a Helvetica or Arial face, largest on `issue9243.pdf`, which is nothing but capitals. The
advances are untouched by it. **ADR 0267 declines to close it**, because moving 0.687500 to 0.729167
is moving to where another program's font happens to sit, and §9.8.1's "[t]hese font metrics provide
information that enables a PDF processor to synthesise a substitute font or select a similar font
when the font program is unavailable" states no `shall` about doing so. `/CapHeight` is on §9.8.1's
ledger row's list of Table 120 entries this tree does not read, and this is the number that list was
missing.

**And the ambiguous bucket's step 7 was run over this list for the first time.** Our ink minus the
lightest live reference's, over all 68 contradicted pages, from artefacts already on disk: the head
is `issue5751.pdf` at **−5.115**, a page this tree already reports and draws nothing on, then
`issue4436r.pdf` −2.203 (`CONTRADICTED_SUBPIXEL_IMAGE`), `issue9243.pdf` −1.549 and
`smask_luminosity_oob_transfer.pdf` −0.779 and `issue7580.pdf` −0.482, and **nothing else past −0.4**.
The positive side is `issue11740_reduced.pdf` +13.704 and `issue14802.pdf` +9.982, both of them a
reference that drew nothing. **Nothing unexplained anywhere on the contradicted list**, which is the
statement `doc/todo/00` records for the ambiguous bucket and which had never been made about this one.

**Three more were re-derived in the four-hundred-and-forty-third, and two of them were the harness.**
`CONTRADICTED_PAGE_ROUNDING` had held `colorkeymask.pdf` and `issue21346.pdf` since the sixth session
on the sentence "we and `ghostscript` produce a raster of one size while `poppler` and `mupdf` produce
one a pixel wider". Rendered through `examples/render_at` at scale 1, **our own rasters are 596 × 842
and 179 × 179** — `poppler`'s and `mupdf`'s sizes — because `TargetSpec::for_page` rounds a fractional
page *up*, which §10.7.4's ledger row has said since the sixty-first session. What was being read is
`<stem>-p<n>-ours.png`, which is our raster **after `normalise::to_common_size` cropped it to the
smallest voting reference's size**, beside reference PNGs that come from the cache uncropped. Checked
rather than argued: our render cropped to the reference's size is byte-identical to the artefact on
both pages. **The group is empty and both pages are diagnosed by what they differ by:**

- **`colorkeymask.pdf` is §10.7.4's image paragraph, and we are the ones who are right.** One
  200 × 267 image at one device pixel per source sample; ours is **byte-identical to `ghostscript`
  over 595 × 842** and `poppler` — which votes with `mupdf` — differs from both on 942 pixels of
  500 990, in three one-pixel columns and one row. Device row 17's centre is outside the region so
  the clause paints nothing there and we paint nothing there; device column 78's centre maps to
  source sample 60, which the file states as `(0, 255, 0)`, and `poppler` paints `(130, 201, 77)` —
  "[t]here shall not be averaging over the pixel area", at the one placement where there is nothing
  to average. `CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`, the list's second entry of that shape.
- **`issue21346.pdf` is six coincident clip boundaries multiplied.** A `W n`, three `/BBox` clips,
  the mark's own path and a mask group's, all stating the same device rectangle; the edge is painted
  at 0.041 where the geometry is 0.827 and §10.7.4's clipping paragraph — a *set* of pixels
  intersected with a *set* of pixels — is 1.000. A ladder of `n` coincident clips reads 0.5025,
  0.2487, 0.1218, 0.0609, 0.0305, 0.0152. `CONTRADICTED_COINCIDENT_CLIP_EDGES`, left listed with the
  price of `min` measured (ADR 0279, `doc/todo/11` item 4).

**And the ranking's own head came off**: `issue15716.pdf` at **3.10 from the nearest against 3.92
from the furthest**, the tightest ratio on the list that is not a link border, is
`CONTRADICTED_SUBSTITUTED_FONT`'s third mechanism and the first with a closed form. Every renderer
paints the area *its own* ZapfDingbats states to a fifth of a percent — ours 12 511.9 px² against
`FoxitDingbats`' 12 520.7, `poppler` 15 294.5 against `D050000L`'s 15 282.0 — with identical advances
(626, 694, 595, 776, Adobe's published metrics) and one of the four glyphs shared outright. A
substituted serif costs nothing measurable, a substituted sans costs `/CapHeight`, and a substituted
symbolic face costs a quarter of its ink.

**Step 7 over the contradicted list reproduces the table above to the thousandth**, twelve rounds of
pixel-moving work later — same five negative names in the same order, same two positive ones — which
is what that alarm is for.

**And the ranking the gate prints is not the ranking `doc/habits.md` asks for**, which the
five-hundred-and-fourteenth session found by taking the other one. `rank_the_contradicted` orders by
*distance from the nearest reference*, borrowed unchanged from the ambiguous bucket; habits says to
rank by **our worst measurement over the bound it is held to**, and the two disagree at the head. By
ratio the first and twelfth of the sixty-eight were `xobject-image.pdf` page 1 at **127.75×** and
`issue5751.pdf` page 1 at **12.66×**, each failing all four bounds — and **neither was on the printed
ranking or in any group in `oracle.rs`**, because both are pages this tree *reports* and
`check_the_ratchets` filters on `complete`. That filter is right and its argument is the oracle
module's own; what nobody had noticed is that it also kept the list's largest disagreements out of
every diagnosis in the file. Both are diagnosed now — the second was a defect and is fixed, the first
is `CONTRADICTED_ON_A_PAGE_WE_REPORT` — and the ranking itself is left as it is, with the reason in
ADR 0349. **Take both orderings: one names the page the references are furthest from, the other names
the page furthest outside what it is held to.**

**And where a corpus states an invariant about itself, that invariant can be asked of the
references** — which is the method the five-hundred-and-forty-sixth session took to the head of the
ratio ranking and is the one way out of "their agreement proves nothing" that does not need anybody
to be treated as truth. The pdf.js corpus's `bitmap-*` family is one drawing encoded through nearly
every path ISO/IEC 14492 defines, so every renderer owes the same picture on all of them, and each
one can be **compared with itself**: this tree returns one image, `poppler` eight, `mupdf` six,
`ghostscript` six — and the image `jbig2dec` produces on the encodings it is consistent about is
byte-identical to ours. A group whose whole argument was *why their agreement is not evidence* now
says what is true. ADR 0381, and the same ADR records the population hole it found: a member of that
family whose name does not begin `bitmap-`, sitting on the contradicted list with the one test that
could judge it filtering it out. **Ask what a population's filter is made of, and which members were
named after something else.**

### 3d. `no render` is the one verdict reached without asking the references

**Taken in the five-hundred-and-seventy-fifth session** (ADR 0410), which went looking for the
robustness question rather than the ledger's and found that the bucket where a defect is *worst* was
the bucket nothing watched.

`examine` returns as soon as `render_ours` fails, so on a `no render` page the three reference
renderers are never invoked. Every other verdict this gate reaches is a statement about a
comparison; that one is a statement about us alone — and it had **no ratchet in either direction**,
so a change that stopped a document opening would have printed one more line in a report of 888 and
failed nothing. `tools/state.sh oracle` prints today's count; `doc/HANDOVER.md`'s trap 1 has called
it "a to-do list of pages nobody has looked at" since the hundred-and-seventy-seventh session, and
until this round nobody had.

**The recipe, and it is cheap** — the references are asked by hand, with
`tools/pdfref/src/reference.rs`'s own invocations copied verbatim so that every one of them is
explicit about the page box, because trap 3 binds a measurement taken outside the harness exactly as
it binds one inside it:

```sh
pdftoppm -r 72 -png -f N -l N -singlefile -cropbox -aa yes -aaVector yes <file> out
mutool  draw -b CropBox -r 72 -o out.png <file> N
gs -q -dNOPAUSE -dBATCH -dSAFER -sDEVICE=png16m -dUseCropBox -r72 \
   -dGraphicsAlphaBits=4 -dTextAlphaBits=4 -dFirstPage=N -dLastPage=N -sOutputFile=out.png <file>
```

then `magick identify` each output and take its ink. **Read the stderr as well as the raster**: three
of the answers here are a renderer printing why it refused, and two are a renderer producing a sheet
of *zero ink*, which is not a page and must not be counted as one.

Four things the first whole run said, and the second is what made the round:

- **Most of the bucket is the standard working.** Eight pages are §7.6.4.1's password, which this
  gate supplies none of, and all three references refuse each of them in the same words — the
  evidence that the empty user password is being correctly *rejected* rather than our derivation
  failing. Two are encryption ISO 32000-2 states no algorithm for. Six are a page tree that yields
  nothing, which `tests/corpus.rs` documents one file at a time.
- **One page was ours, and it is the whole reason to ask.** `boundingBox_invalid.pdf` page 1 states
  `/MediaBox [0 0 0 0]`; `poppler` and `mutool` draw it at 612 × 792 and this tree drew **nothing at
  all**, because §7.9.5's NOTE makes that array a rectangle and nothing downstream asked Table 31
  whether it is a medium. ADR 0410 and §7.7.3.4's ledger row.
- **One page is the *gate* refusing, not the program**, and the verdict could not say so.
  `issue19517.pdf` is 12 608 × 16 806 at one pixel per point, past `PIXEL_BUDGET`; drawn through
  `examples/render_at` with the interpreter's own bound it is ink **172.597** against `pdftoppm`
  172.602, `mutool` 172.599 and `gs` 172.599 — agreement with all three to **0.005 of 255** on a page
  this gate has never judged. The budget stays where it is; what was wrong is that the bucket it
  lands in named the program. **A verdict that accuses the program when the instrument is what
  declined is the shape to watch for.**
- **One page is two references implementing an unpublished extension.** `Brotli-Prototype-FileA.pdf`
  is drawn by `mutool` and `gs` and by neither `poppler` nor this tree; its object streams use
  `/BrotliDecode`, which ISO 32000-2 does not define, so two renderers agreeing about it is not
  evidence about a clause.

**The bucket is held by name now**, in `oracle.rs`'s `NO_RENDER_*` groups, over *all* pages rather
than the complete ones — a page that renders nothing is never complete, so a list filtered on
`complete` would hold nothing against nothing. A page arriving is a page that stopped being
comparable at all; a page leaving has been fixed or has to be deleted from its group.

### 3c. The bound those thirty-eight fail was never derived, and it is left where it is

**The four-hundred-and-seventh session asked the question §3b's last paragraph raises** — a bound a
page fails while passing every other bound comfortably is either the bound doing its job or a bound
nobody derived — **and measured it the way this file's structural floor of 0.90 was measured, by
asking how far the references sit from each other.** `oracle.rs`'s
`the_fixed_bounds_against_the_references_own_spread` re-derives all eight fixed bounds from the
corpus: every reference pair on every page, split by tolerance class and by whether the pair crosses
the hinting boundary, **each measure taken over the pairs the *other three* bounds admit** — which is
`Tolerance::VECTOR`'s own stated method and is what stops a bound from being measured over the
population it already defines.

**Over 2638 pairs of the three independent references on text pages, the share of reference pairs
each `TEXT_HEAVY` bound rejects is 0.0% (mean), 1.2% (worst tile), 0.5% (structural similarity) and
29.4% (the differing fraction).** One bound of the four sits below the spread of the implementations
that set it, and it is the one 38 of the 68 fail on and no other. The same measure on *vector* pages
rejects 2.8% against its siblings' 0.0%, 9.7% and 3.3%, so this is one number in one class rather
than an argument about counting channels.

**The sentence that claims to derive it names another measure's number.** Re-run on the population
that sentence cites — the 14 specification PDFs' first pages, 42 reference pairs — the worst tile
reproduces to the digit (p90 26.72, max 28.17, "26 to 28"), and the differing fraction is median
3.11%, max **5.14%**, with **11.9% of those pairs already outside the 5.00%** the comment sets. What
is 2.7 on that population is the *mean absolute difference*'s maximum, 2.7355.

**It is not moved, and both reasons are measured.** The bound decides whether two references form a
consensus *and* floors the per-page bound; raised to the reference spread's 99th percentile, 12.02%,
the corpus goes from 905/68/786 to **1121/309/329** — 457 pages leave `ambiguous` and **278 arrive
newly contradicted** against 37 leaving, which is 278 diagnoses rather than a round. And the only
population that would justify loosening our own side alone is the one crossing the hinting boundary,
where the median differing fraction doubles (1.69% → 3.42%) — but the sole renderer on the far side
of it is `hayro`, which shares `skrifa` with this tree, so it is not evidence about us.
**ADR 0243** has the tables; [todo 12](todo/12-one-bound-two-jobs.md) has the work; the 38 pages'
status is unchanged and now has a reason beside it.

**Two cautions the contradicted list earned.** A page may be contradicted for a reason other than
the one its group names — **ten for ten, so far, on the group being wrong**. (The tally did not move
in the five-hundred-and-forty-sixth session, which re-opened the ratio ranking's head — the seven
JBIG2 pages, the three link borders, the two the references did not draw and the `DeviceCMYK` ramp
— and found every *diagnosis* right. What was wrong on those pages was the evidence inside three of
the notes: a log generalised from one page to seven, a claim about `ghostscript`'s log that is only
true without the `-q` the gate passes, and a fourth renderer's silence nobody had recorded. **A
sweep that confirms is not a sweep that found nothing**, and the distinction between a wrong label
and stale evidence under a right one is the thing to report.) The newest is
`issue9940.pdf` in the five-hundred-and-fourteenth, whose `CONTRADICTED_CALIBRATED_COLOUR` had said
that `mupdf` and `ghostscript` take a `CalRGB`'s components for `DeviceRGB`; a swatch carrying that
file's own `/CalRGB` dictionary says nobody does, ours and `poppler`'s reproduce §8.6.5.3 plus IEC
61966-2-1 exactly, and the page is `CONTRADICTED_CALRGB_TO_SCREEN`'s §10.3.1 (ADR 0349). The one
before it is
`calrgb.pdf` pages 1, 5, 11 and 12, which sat in `CONTRADICTED_SUBSTITUTED_FONT` from the sixth
session under a note naming *another* group's mechanism, and which differ from one another only in
a `/BlackPoint` no voting renderer reads (ADR 0296) — the tell being that all four printed metrics
were identical on all four pages, which the gate had been saying every run. The one before it is
`issue4304.pdf` in the four-hundred-and-fifth session, which sat in the same group
for a hundred and eighty sessions while drawing *Wordsthatshouldhavespacesbetweenthem.* against
four renderers' *Words that should have spaces between them.* Its font really is substituted and
that really was not the difference: `/Differences [32 /.notdef …]` sent code 32 to a glyph whose
advance §9.6.2.1 obliges this processor to supply, and the third of `simple_widths`' three sources
read the program through an sfnt parser that refuses the bare CFF ten of the fourteen compiled-in
standard faces are. Six spaces of zero width — and "make it match
mupdf" is the failure principle 5 forbids. And a page can be contradicted by a departure this
project decided on purpose: `colors.pdf` pages 1 and 2 left the unexplained list in session 68 and
are *not* fixed, because §10.7.4 asks for the hard edge and this tree anti-aliases
(`CONTRADICTED_ANTIALIASED_EDGES`, and `doc/todo/_scan-conversion.md`).

**The incomplete documents** — `tools/state.sh corpus` counts them, and this paragraph does not, because it twice carried a number the gate had moved past: it said **67** against a gate printing 68 for four rounds, and **70** against a gate printing 65 for a further twenty-six. What is worth keeping is which documents joined and why. Two joined in that session and are a *new report* rather than a regression (trap 5): `issue6541.pdf` and `issue8702.pdf`, each of which names an `/XObject` its own resource dictionary does not define, and neither of which loses a mark by it — the first because the object it cannot reach is an empty stream, the second because the object carries no stream at all. A third document gained the same report inside a list it was already on (`operator_list_cycle.pdf`, a `gs`). ADR 0255. The history below is what the number was before that: 68 until the four-hundred-and-nineteenth, and — 70 until the three-hundred-and-ninety-seventh, which stated a knockout element's shape apart from its alpha and took `knockout_nested.pdf`, `knockout_nested_group_alpha.pdf` and `knockout_smask.pdf` off the list with nothing joining (ADR 0234); **this paragraph said 73 for four rounds after the three-hundred-and-eighty-third took it to 70, and the three-hundred-and-eighty-seventh counted them off the gate rather than off this file** — 73 until that round's second residue of §11.5.3 landed (ADR 0220), 72 until the three-hundred-and-eightieth, whose one new report is a `/DeviceN` shading inside a `/DeviceGray` luminosity mask group (ADR 0217), 74 until the three-hundred-and-fourteenth and 76 until the two-hundred-and-eighty-second, where a `Tf` naming
`/Helvetica` with an empty resource dictionary stopped meaning nothing, because §9.6.2.2 says those
fourteen names name something every processor has (ADR 0183). **The split below was counted off
the gate's own output in the three-hundred-and-eighty-seventh** and is by report kind, which is
what the gate prints: **29 fonts** (fewest since session 6 — session 156's `CMap`s took 15 off this
list — of which 4 report a font program that draws nothing, ADR 0157), **10 transparency** (8 a
group, 2 `CompositedInParts`), **10 operator soup** (`BT` without `ET`, `BDC` without `EMC`, fuzzed
streams), **7 malformed images**, **6 annotations** — Table 179's line endings took one in the
three-hundred-and-fourteenth (ADR 0192) — **3 a budget reached**, **2 an undecodable content
stream** and **1 a shading**. `doc/todo/23` says what each of the transparency populations now
owes, and its own count is the 8 documents: 19 before the three-hundred-and-eightieth,
14 after it, ADR 0220 took the three that close it to 11, and ADR 0234 took four more. Session 59's reading of
the corpus's own issue trackers says most of the font half is glyph rasterisation on files chosen
for having hard fonts, which session 68 then measured on one.

### 3a. The ambiguous bucket — watched since the hundred-and-seventy-sixth, and emptied in the three-hundred-and-seventy-ninth

**749 of the pages the oracle judges on documents we call complete come back `ambiguous` (786 of
all 1794), and until the hundred-and-seventy-sixth session no gate watched one of them.** **0** are
still undiagnosed, from 754, since the three-hundred-and-seventy-ninth session — and the instrument
is not retired by that: the gate holds the list to equality in both directions, so a page that stops
agreeing arrives in an empty file and fails the build on the arrival, which is the regression it was
built to see. Step 7 — our ink minus the lightest live reference's, over every ambiguous page — is
the half no ranking can do and stays standing.

The count in this file used to be 72, which was `wc -l` of a file with a twelve-line header and was
corrected in the three-hundred-and-seventeenth by counting what the gate counts. The twenty rounds
from the two-hundred-and-fifty-first took three populations at once and then worked the tail a page
at a time. The verdict means "nobody's difference is large
enough to call anybody wrong", which is the right thing for the *ratchet* to do and is not the
same as "right". `issue7406.pdf` drew a JPEG cyan-on-black inside an `ambiguous` verdict for as
long as anybody looked, and it is correct now, and **nothing announced either event**.

The project owner's judgement, in the hundred-and-seventy-fifth session, is that the tree is far
enough along for this to be the work rather than a caveat. It is the last large population where
a defect can live without a name, and **the task, the instrument, the method and the next names
are [todo 00](todo/00-ambiguous-bucket.md)**.

**What it has produced, because that is the argument for keeping at it.** Forty-five sessions,
**fourteen defects found and thirteen of them fixed** — the newest being a page this tree drew
*nothing* on, which the ranking rated 0.73 and the step-7 sweep found at −1.783 (§12.5.6.4's text
annotation attached to a point) — — a page one that was page two (ADR 0148), a
photograph rendered black (0149), a shading painted as a square (0150), a stencil that drew
nothing (0151), a whole grid that disappeared (0154), a sentence drawn as one Greek letter
because the font's name ends in the word "Symbol" (0158), a stamp's gradient painted flat
(0160), a widget's border losing a fifth of its ink to a clip on its own edge and a comb field's
separators losing theirs to a miter bound (0165), a `loca` whose offsets descend so that 36 of
one font's 71 glyphs were refused in silence (0170), **§8.7.4.5.4's greatest *admissible*
root** — found in the two-hundred-and-sixth session, fixed in the two-hundred-and-thirty-second
on all three backends at once (0171), and the longest-standing of them because every gradient
library gets it wrong the same way — **a blurred word nobody drew** (0173): §8.6.8's
uncoloured restriction was still in force inside a soft mask's own group, so a `d1` glyph
procedure that set a `/Luminosity` mask had its mask evaluated to zero and painted nothing, with
every command present and nothing reported — and **a space that was a bar** (0174), where the
`loca` repair of sixteen sessions earlier read a glyph's length from its own bytes even where
the table said, in the standard's own spelling, that the glyph was empty.

Beside them: a pattern cell's clip worth 15% of a page's ink (0155), ten documents whose
substituted font drew none of its characters in silence (0152), the coverage rule that made
eight of them draw (0153), and a font program that draws nothing now saying so (0157).

**The eleventh is found and not fixed**, from the two-hundred-and-fifteenth: a stroke under a
pixel wide loses the half of `tiny-skia`'s hairline smear that falls outside the raster's top
edge, so `vertical.pdf`'s two hairlines carry 55% of their area at the page's top and 98%
everywhere else ([todo 11](todo/11-shapes-that-still-disappear.md) item 3). The bucket itself
went 754 → **0** undiagnosed and all 786 pages carry a diagnosis; *eleven defects nobody could see* is
the number to watch.

**And the three-hundred-and-seventy-ninth took the last five, none of them a defect either, and
each by a different mechanism** — two on §10.7.4's glyph edges where `issue4665.pdf` is the first
page in the bucket on which *all four* references converge on one number (four ladders within 0.044
of 255, three within 0.009); one on §9.7.4.2's own closing sentence, with the half that clause does
**not** leave open checked at 8× to the pixel; one where 111 of a Type 3 font's 114 glyph
descriptions paint themselves white and §9.6.4 Table 111 takes the colour away, so the two readings
differ by a blank page; and one where `ghostscript` prints *An embedded font is invalid* and
substitutes, with the corrupt part of that CFF measured to be the Private DICT's hinting operands,
which carry no outline. **The instrument it added is for a ladder that does not converge**: a
reference's excess divided by the ink a one-pixel erosion removes is an outward offset, and
`ghostscript`'s triples in device pixels while holding at 0.040 ± 0.004 *points* — user space, so a
different shape rather than a different sampling.

**And the three-hundred-and-seventy-second took three names with no defect among them, which is
the outcome worth describing anyway** — because two of the three replaced a group's *argument*
with **arithmetic**. `bug1889122.pdf` is one stroked rectangle whose ink can be written down
(`150 × 22 − 148 × 20 = 340` square points over 19 635 pixels, 4.4156 of 255), and ours is 0.05%
over it where `ghostscript` is 26.7% over and `hayro` 17% under —
`AMBIGUOUS_WIDGET_BORDER`'s sentence for the sixth time and the first time against a number rather
than a limit. `issue4379.pdf` places a stencil-masked image at an exact two-to-one reduction onto
integer device coordinates, so §10.7.4's sampled-image paragraph names one raster sample by sample:
`ghostscript` reproduces it on **all** 500 990 pixels and this tree departs on **3 927**, which is
ADR 0025's stated cost measured on a real page for the first time — invisible to any ink
measurement, since the five renderers agree to 0.023 of 255 there. `issue14953.pdf` declares
`0 0 0 0` for its Type 3 font box and for all fifteen of its glyphs, and a synthetic A/B that
differs only in `d1`'s four operands shows `ghostscript` drawing nothing above 72 dpi and `poppler`
losing the glyphs as the pixels shrink, while this tree and `mupdf` are byte-identical across the
pair — §9.6.4 Table 111's "the result is implementation-dependent" with the implementations
separated. **Its by-product is the round's spec-track item**: §9.2.4's and §9.6.4's ledger rows both
attributed to Table 111 a permission ("a processor may make no assumptions") that Table 111 does not
contain and Table 110 states only for an all-zero *font* box. Both corrected.

**Step 6's own assumption failed for the first time in the two-hundred-and-sixteenth**, on
`issue2177.pdf`: the closed form takes a reference to eight times the resolution because a
renderer's departure from the geometry shrinks with the pixels, and `poppler` on a §8.7.3 tiling
pattern goes the other way — 34.15 → 18.03 → 16.32 from 72 to 2304 dpi, its strokes thinning
rather than its edges sharpening. Ours is flat across four scales and `mupdf` at 8× agrees with
us to four significant figures. **A limit is only a limit if the thing taking it is converging,
and one ladder cannot tell convergence from drift** — take two.

**And the two-hundred-and-fifteenth session cleared the whole ranking above 1.6 in one sitting —
seven pages — which is a result about the *list* rather than about any page.** Two were a face
nobody ships and where §9.8.1 puts the answer, two were one word on a page the size of a postage
stamp, two were hairlines, one was an eight-bit ramp on a stamp fixed sixteen sessions earlier.
**The top of the ranking is populations now rather than defects**, and the one new defect in it
came from a synthetic ladder rather than from a reference.

**And the ninth was a correction rather than a finding, which is why it is here.** Two of the
eight above quoted an ink table that was **half** ours and `hayro`'s and whole for the three C
references, because the method file's own command averaged an alpha channel in — a defect
session 161 found, fixed and wrote down in two places, neither of them the file a session reads
when it goes hunting. Both ADRs carry the correction, the recipe is repaired, and there is a new
closed form beside it: the same page at eight times the resolution, which is what says *which*
renderer is measuring area. ADR 0163.

**And the eighth was a gate rather than a page.** `jp2k-resetprob.pdf` sat first on the ranking
at 5.03 and its name is a JPEG 2000 coding option; checking that hypothesis meant decoding every
`JPXDecode` stream in the corpus against ISO/IEC 15444-5's reference software, which **ruled the
codec out for that file and found thirteen of the thirty codestreams wrong** — every one of them
on the irreversible 9/7 path, by up to 87 levels of 255. Four codecs reach this tree through
dependencies and only two of them had ever been checked against anything. ADR 0161,
`doc/JPEG2000_FEEDBACK.md`.

**The seventh was the ranking's own first name.** `issue7821.pdf` sat at 5.44 bounds with a
stamp whose rounded box looked like a plausible flat green fill and is a shading pattern in four
other renderers: an annotation's appearance stream is a form XObject, and §8.7.2's rule about
where a pattern's matrix points was applied on the `Do` path and not on the appearance path, so
the axis landed off the page and `/Extend` painted one colour. **§8.7.2's ledger row has now been
wrong twice about the same sentence, once per way of becoming a parent content stream.** ADR
0160.

**And the sixth was found by a comment rather than by a number.** `issue8697.pdf` was on the
text gate's list with a paragraph explaining that its readback was a question about §9.10.2 and
that "both readbacks are defensible" — four true sentences about the readback, none of which
asked why the page was in Greek. The defect was in font *substitution* one stage upstream, and
the gate that could see the symptom had closed the question downstream of it. ADR 0158.

**Two of those are worth repeating here because they are about this file rather than about a
page.** `CONTRADICTED_SUBSTITUTED_FONT`'s comment said two documents drew "the same five kana in
the same places in all four panels" — our panel was white, and the sentence described the
*references'* half of the side-by-side. **A group's comment is a claim about a picture, the
picture is one `Read` away, and no gate can check a comment.** And `ambiguous` is not a measure of
how wrong a page is: `issue13372.pdf` sat at 26.95 bounds inside a verdict that cannot tell a
blank page from a grainy one.
