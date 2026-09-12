# Third-party data and dependency decisions

Status: **record** — what was read, off copies on this machine, and what each decision cost.
Read by: whoever is about to vendor data or take a dependency. The licence obligations this
lists are met by `/NOTICE` and checked by `viewer-ui/tests/notices.rs`.

`doc/HANDOVER.md`'s reader table points a round taking a dependency here. ADR 0133 cites "`HANDOVER.md` §1" and means
this.

**This project is Apache-2.0** as of the eight-hundred-and-eighty-seventh session, on the
project owner's word; it was MIT from the hundred-and-thirtieth, and MPL-2.0 before that. One
author in the whole history, so no relicensing here has needed anybody else's consent.
`deny.toml`'s allow-list dropped MPL at the first move and needed no change at the second —
Apache-2.0 has been on it since the beginning, because dependencies were already under it.

**What the move costs the graph is nothing, and that is checkable rather than assumed**: every
package below is MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0, CC0 or BSL, and Apache-2.0 is
compatible with all of them in the direction that matters — a permissive dependency may be
combined into an Apache-2.0 work. Nothing in this tree is under GPL or MPL; the one GPL item
this record names is poppler's `cidToUnicode` data in the table below, which was examined and
**not** taken. `cargo deny check licenses` is the command that keeps that true.

**One obligation is new and it is met by a file that already existed.** Apache-2.0 section 4
requires a redistribution to carry the licence text and, where the work has one, its NOTICE
file. `/LICENSE` and `/NOTICE` are both at the root and CI packages both beside every binary,
which is what the BSD-3-Clause font programs already needed.

**All three items shipped**, in the order this section used to prescribe, and the table stays
because it is the record of what was read — off copies on this machine, not recalled:

| data | source examined | terms |
|---|---|---|
| Adobe predefined `CMap`s | `poppler-data`'s `COPYING.adobe` (1990–2019), `doc/pdf.js/external/bcmaps/LICENSE` (2009), `hayro-cmap`'s `assets/LICENSE.txt` (2023) | **BSD-3-Clause** |
| Foxit standard-14 programs | `doc/pdf.js/external/standard_fonts/LICENSE_FOXIT`, from PDFium | **BSD-3-Clause** |
| Liberation Sans | `LICENSE_LIBERATION` | **SIL OFL 1.1** (reserved font name: ship and use freely, do not modify and keep the name) |
| poppler's `cidToUnicode`, `nameToUnicode`, `unicodeMap` | `poppler-data`'s `COPYING` | **GPL-2 or GPL-3** — Glyph & Cog's, *not* Adobe's |

`BSD-3-Clause` costs three obligations: reproduce the notice and disclaimer "in the documentation
and/or other materials provided with the distribution", keep them in source, never use Adobe's or
Google's name to endorse this. **The surface for that is `/NOTICE`** — at the repository root,
`include_str!`d by `quorra --licences`, put over the page by `?` as the About panel, set in
§9.6.2.2's own Courier and deliberately *not* re-wrapped, because re-flowing text a licence
obliges this program to reproduce would be editing it. `viewer-ui/tests/notices.rs` checks that
every `.pfb` and `.ttf` under `data/` is named **by file name**, that the required sentences are
verbatim, and that the bytes still hash to `SHA256SUMS` — that test exists because `cargo deny`
reads Cargo metadata and **cannot see vendored data**.

**The trap is the last row.** `poppler-data` is two data sets under two licences and says so. A
`CMap` gets code → CID; getting a CID to a glyph in a **non-embedded** CJK font needs CID →
Unicode, which is the GPL half. The permissive equivalent is Adobe's own `Adobe-Japan1-UCS2`,
`Adobe-GB1-UCS2`, `Adobe-CNS1-UCS2`, `Adobe-KR-UCS2` — BSD files inside the `cMap` directory. For
an *embedded* CIDFont none of it is needed: the font's own charset or `/CIDToGIDMap` answers.

**What shipped**: `data/standard-fonts/` holds §9.6.2.2's fourteen font programs, 804 KB, so
those pages reproduce on any machine and `substitute.rs` is no longer the only machine-dependent
code in the tree (ADR 0133); `data/cmaps/` holds all 239 `CMap`s Adobe publishes, deflated one at
a time by `build.rs` into a 3.9 MB blob and inflated only when a document names one, so nothing is
decompressed at startup (ADR 0140). §9.10.2's third method came with the second, and it is where
the gates moved: 15 documents left the incomplete list, 9 more oracle pages agree, the readback
went 97.9% → 98.2%.

**Three things worth keeping out of the first**: PDFium's `.pfb` files are bare CFF programs
(`01 00 04 02`), a name-keyed substitute is addressed by glyph *name* and so needs no Adobe Glyph
List step — which is why `Symbol` and `ZapfDingbats` work — and a *composite* font cannot use them
at all, because §9.7.4.2 leaves it reachable only through `/ToUnicode`, which addresses by
character.

**What none of it fixed** is the 40 fonts naming an `Identity` ordering, where the codes index a
font nobody supplied: [todo 21](todo/21-font-substitution.md).

**And a todo file's claim decays exactly as a ledger row's does, with no sweep watching it.**
That file named two documents whose "characters no single face on this machine has" and the
two-hundred-and-fifty-sixth session opened the pictures: both draw every character, and had since
ADR 0153's coverage rule landed seventy-three sessions earlier. The claim was a *prediction* about
that rule which nobody re-checked after it shipped. `doc/todo/01`'s five sweeps read `ledger.toml`
and `crates/`; **`doc/todo/` is a third population and is watched by nothing**.

**And one dependency decision, taken in the three-hundred-and-seventy-sixth session** —
`accesskit` 0.24.1 and `accesskit_unix` 0.22.1, both **MIT OR Apache-2.0**, with **61 packages**
behind them, every one MIT, Apache-2.0, Zlib or Unlicense-or-MIT. `cargo deny check` is clean on
all four with no new exception in `deny.toml`. Two things about it are worth keeping here rather
than in ADR 0214: **`memchr` is not among them at runtime** — it arrives through `winnow` and
`proc-macro-crate`, which are a proc macro's build dependencies, so the rule ADR 0186 refused
`quick-xml` over is intact and `cargo tree -e normal` is how that was checked; and **the one async
runtime in this tree is `accesskit_unix`'s**, which lives on a thread of its own, is named by one
crate, is `cfg(target_os = "linux")` in that crate's manifest, and is not created until the first
frame has been presented.

**And a second, in the three-hundred-and-seventy-seventh** — `sha1` 0.11.0 and `ripemd` 0.2.0, both
**MIT OR Apache-2.0**, from the same RustCrypto family as the `sha2` and `md-5` §7.6 already brought
in, **two new packages between them** and no transitive dependency this tree did not already build
(`cargo tree -e normal` shows only `digest`, `cfg-if` and `cpufeatures`, all of which `sha2` already
pulls). They exist because §12.8.3's Table 260 and Table 256 name six digest algorithms between them
and the four already here are not all six; implementing five and being silent about the sixth is the
failure this project spends its rounds removing. `cargo deny check` clean on all four with no new
exception (ADR 0215).

**And in the three-hundred-and-ninety-second, a dependency decision that came out `no`** — the
first in this record that adds nothing. Answering §12.8.1's *second* question needed an X.509
certificate reader and an RSA verification, and `rsa`, `p256`/`p384` and `x509-cert` were all
declined: `rsa` 0.9.10 brings **31** packages including a second `digest` 0.10 hash stack beside
this tree's 0.11, `rsa` 0.10 is a release candidate, `p256`+`p384` bring **28** and cover two of the
five curves RFC 5480 names, and `x509-cert`'s strict DER would refuse the four corpus signatures
that state indefinite lengths. Both modules are in tree — `pdf_model::x509` at 213 lines of code and
`pdf_model::pkcs1` at 356 — under `#![forbid(unsafe_code)]`, and **`cargo deny check` is clean on
all four checks with no package added and the licence position unchanged**. ADR 0229 has the
argument, including what would change the answer: an ECDSA or DSA signature arriving in a real file.

**And in the four-hundred-and-eighth, `gtk4` 0.11.4 — a *toolkit*, which is a first for this
record.** `doc/todo/30`'s order made GTK4 the first native host, and the crate is the GNOME project's
own GIR-generated binding. **41 packages** arrive with it — `glib`, `gio`, `gdk4`, `gsk4`, `pango`,
`cairo-rs`, `graphene-rs`, their four `-sys` crates, and the `system-deps`/`pkg-config` machinery
their build scripts use — and **every one of them is MIT**, which was this project's own licence
when this paragraph was written and is compatible with the Apache-2.0 it now is, so the answer to
"what may I do with a build of this?" is unchanged. `cargo deny check` is clean on all
four checks with no new exception in `deny.toml`.

Three things about it belong here rather than in ADR 0244. **It is confined to one crate**: `gtk4`
is named by `crates/viewer-gtk`'s manifest and by no other, nothing that touches PDF bytes gains a
toolkit dependency, and `viewer-ui`'s and `viewer-confined`'s graphs are byte-for-byte what they
were. **It costs no `unsafe` in this tree**: `viewer-gtk` and its binary both keep
`#![forbid(unsafe_code)]`, because `gtk4-rs` is a safe binding and the `unsafe` is inside
`gtk4-sys` and `glib` — which is the property `doc/todo/30` chose GTK4 for and the property the Qt
host will not have. And **it is linked against the platform's own libraries** rather than vendored:
a native host depending on the desktop being installed is the point of a native host, and it is why
`viewer-gtk` is excluded from the two cross-target checks by the same rule that makes
`viewer-accessibility` Linux-only in its own manifest — `glib-sys`'s build script wants a
cross-compiling `pkg-config` and there would be no GTK 4 development files for the target anyway.

**And in the four-hundred-and-tenth, `cxx` 1.0.199 and `cxx-qt-build` 0.9.1 — a *C++ bridge*,
which is a second first for this record.** `doc/todo/30`'s order put Qt second precisely because it
costs one, and this is what the one costs. **23 packages** arrive, and **three of them reach a
shipped binary**: `cxx`, `cxxbridge-macro` and `link-cplusplus`. The other twenty are build-time
only — `cxx-qt-build`, `cxx-gen`, `qt-build-utils`, `clang-format`, `which`, `codespan-reporting`
and the rest — and appear in no `cargo tree -e normal`. Every one of the 23 is MIT, Apache-2.0, or
Unlicense-or-MIT, all already on `deny.toml`'s allow list, so the answer to "what may I do with a
build of this?" is unchanged again; `cargo deny check` is clean on all four checks with **no new
exception**.

Four things about it belong here rather than in ADR 0246. **It is confined to one crate**: `cxx`
and `cxx-qt-build` are named by `crates/viewer-qt`'s manifest and by no other, nothing that touches
PDF bytes gains either, and `viewer-ui`'s, `viewer-gtk`'s and `viewer-confined`'s graphs are what
they were. **`cxx-qt` itself was declined**, which is the part worth recording as a decision: the
crate that makes a Rust type into a `QObject` is built for QML and links two initialisers a Widgets
host never calls, and only the *build* half — finding Qt through `qmake6`, running `moc`, linking
QtCore/QtGui/QtWidgets — is wanted, and it is available on its own. **It costs one hand-written
`unsafe` token**, the `unsafe extern "C++"` block header, which is `cxx`'s way of asking the author
to assert that the C++ declared there exists and is safe to call; `viewer-qt` holds
`#![deny(unsafe_code)]` with one exemption on `mod bridge`, and `tests/unsafe_position.rs` asserts
the file and the token and — since the four-hundred-and-eleventh, when `viewer-ffi` arrived and the
test was amended rather than loosened — that **exactly two** crates in the tree lift the denial
(ADR 0247). The second took **no** dependency at all: its header is hand-written, so there is no
`cbindgen` in this record and nothing new in `deny.toml`. **And it is linked
against the platform's own Qt** rather than vendored, for the reason `viewer-gtk` links against the
platform's GTK: that is what a native host is, and it is why `viewer-qt` is excluded from the three
cross-target checks — `cc-rs` wants `lib.exe` for a Windows target and there would be no Qt 6
development files there anyway.

**And in the four-hundred-and-twenty-second, three *corpora* — which is a third first for this
record, because a submodule carries no bytes into this history and the licence question is
therefore about what may be *promoted* out of one rather than about what may be shipped.** The
positions, each read off the repository's own licence file rather than off GitHub's guess:

| submodule | licence, as examined | what it permits here |
|---|---|---|
| `doc/corpora/pdf20examples` | `LICENSE.md`: "The PDF Association provides these example PDF 2.0 files under the Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0) license." | submodule freely; a file **copied** into this tree would carry attribution and the licence with it, and CC BY-SA 4.0's ShareAlike condition attaches to *Adapted Material* rather than to a Collection, so a verbatim copy beside this tree's Apache-2.0 code does not relicense the code |
| `doc/corpora/pdf-differences` | `LICENSE`: the Apache License 2.0 in full — **and the root `README.md` splits the repository in two, which this row read as one until the five-hundred-and-fifty-eighth session**: "PDF files are copyright by the PDF Association and distributed under a [Creative Common Attribution 4.0 International (CC BY 4.0) license](https://creativecommons.org/licenses/by/4.0/). Any source code in this repository is licensed under [Apache License, Version 2.0]" | submodule and copy freely; a **PDF** copied out carries CC BY 4.0's attribution rather than Apache-2.0's notice, and the `README.md` beside each test case is where the attribution comes from |
| `doc/corpora/pdfbox` | `LICENSE.txt` Apache-2.0, with `NOTICE.txt` beside it | as above, and `NOTICE.txt` is the file §4(d) obliges anyone redistributing to carry — which is why the sparse checkout keeps the repository root rather than the test directory alone |

**Nothing was copied**, so none of those obligations is live: `crates/` holds no `.pdf` at all,
before this session or after it, and the one witness this round promoted is named by its path
inside a pinned submodule. That is the cheapest possible answer to a licence question and it is
worth stating as the rule rather than as an accident.

**`openpreserve/format-corpus` was examined and declined** in that session, and on licence rather
than on size: GitHub detects none; its `README.md` says "[a]ll items are CC0 licenced **unless
otherwise stated**", and per-file `.md` sidecars are where such a statement would be — a grant with
an escape clause is not one this project relies on without reading it, which is ADR 0187's
discipline. The four-hundred-and-sixty-seventh read every sidecar
(`doc/oracle-and-corpus.md` §2c) and put the question to the project owner, **who reversed the
caution rather than answering the file**:

> Add as many submodules as you want unless we can clearly deduce from their licence that we are
> not allowed to do so. We don't even republish, so most licence notes don't even apply to us!
> (We should still mention them as a courtesy.)

That is a rule rather than a ruling, and its two halves are what the fourth corpus was taken
under. **A submodule is a pin, not a copy**: this repository records a URL and a commit
identifier, so a clone fetches those bytes from *their* server under *their* terms and this
history carries none of them — which is why the redistribution conditions that dominate the table
above are not engaged at all. What remains is the courtesy, and it is met here and in `/NOTICE`
§3. ADR 0305.

**So `doc/corpora/format-corpus` is the fourth**, pinned at `366f068c` and sparse-checked out to
three of its five PDF directories. Each row is what that directory's own sidecar says, quoted, and
where it says nothing this table says so rather than implying terms it does not carry:

| directory | what its own files state | why it is here |
|---|---|---|
| `pdf-handbuilt-test-corpus` (89 files, 360 KB) | **nothing of its own**, so the repository's root default applies; its `README.md` credits an iPRES 2017 paper and a deposited artefact, DOI [10.22000/53](https://dx.doi.org/10.22000/53) | the reason for the whole exercise: 89 files carrying **one** deliberate structural defect apiece, all drawing the same *Hello PDF-world!*, so a blank page is a finding without a reference (ADRs 0302, 0303, 0305) |
| `pdfCabinetOfHorrors` (24 files, 9.7 MB) | **CC0, explicitly**: its `readme.md` ends "All files in this folder: Creative Commons CC0: Public Domain Dedication." | archival horrors — encryption at three permission settings, embedded video, a corrupt byte, a JPEG whose `/Height` was altered |
| `govdocs1-error-pdfs` (54 files, 63 MB) | **otherwise stated, and permissively**: "All PDF files in this folder and subfolders are copied from Govdocs1", quoting Govdocs1's own "freely available for research and may be (to the best of our knowledge) freely redistributed" and asking for a citation — Garfinkel, Farrell, Roussev and Dinolt, *Bringing Science to Digital Forensics with Standardized Forensic Corpora*, DFRWS 2009, which this line is | `.gov` documents from the 1990s and 2000s that broke somebody else's software: legacy producers, four unparsable CFF programs and a truncated `head` table between them |

**Two directories were left, and neither for a licence that forbids** — saying so is the point,
because the owner's rule turns on *clearly forbids* and neither of these does:

- **`jhove-errors`** (99 files, **275 MB**) — no sidecar at all, and its files are published journal
  articles and theses from Springer, Wiley and university repositories. The root CC0 default is
  unreliable rather than absent here: a third party is in no position to dedicate somebody else's
  paper to the public domain. That is an *absent grant* rather than a prohibition, so under the
  owner's rule it could be pinned; it is left out because it would nearly quadruple what this
  corpus costs a fresh clone. **It used to be left on that *and* on value — "surveying it produced
  two ordinary reports" — and the second half is disproved**: the five-hundred-and-forty-fourth
  session ranked it against three references and one of its 99 files was a 21-page paper this tree
  showed no page of, on a `startxref` eight megabytes from the end of the file (ADR 0379). So it is
  left on size alone, the guard for that defect is a hand-built pair rather than the file, and a
  round wanting the population fetches all five directories into `corpus-cache/` as session 467
  did.
- **`fully-featured-pdf`** (1 file, 23 MB) — no sidecar, and the file embeds third-party media (an
  MP3, a QuickTime movie, a U3D model) the README does not licence. Left on value first: one
  document, already complete, whose distinguishing half is Clause 13, which `CLAUDE.md` excludes.

**What that costs a fresh clone**: 73 MB checked out and about 58 MB of pack, against
`doc/pdf.js`'s 350 MB and `doc/arlington-pdf-model`'s 125 MB. **No gate requires it** — the
submodule is named by `doc/oracle-and-corpus.md` §2b and by `tools/safedocs survey --dir`, which a
round runs on purpose, and by nothing in `doc/todo/02` §2.

**And what `tools/safedocs` fetches is under no grant at all**, which is why `.gitignore`'s entry
for `corpus-cache/` now carries a licence sentence: the `CC-MAIN-2021-31` corpus is eight million
PDFs crawled from the public web, and Common Crawl's own terms of use govern the *collection*
rather than the copyright in each document. Cache them, measure them, name them in a bug report;
do not commit one. ADR 0258's promotion budget exists for the one case where that rule and a
regression test collide, and it has not yet been reached.

**The same sentence binds SafeDocs' *Issue Tracker* corpus, and one more binds it harder.** That
set — under `corpus-cache/tika-issue-tracker/`, fetched for the first time in the
eight-hundred-and-fifty-fifth session, `doc/todo/03` §29 — is bug attachments gathered from 35
issue trackers of 32 PDF technologies. Every file in it belongs to whoever filed the report and
nobody granted anything; Apache hosted it for the tool developers and **stopped**, on takedown
requests, closing the question as `LEGAL-696` in April 2025. So the rule here is the crawl's rule
with its reason made sharper: cache them, measure them, name one in a commit message with its
digest, **commit none of them** — and the one route left to the bytes is the Internet Archive's
copy, each tarball checked against the SHA-512 Apache published beside it, which is what keeps a
third party's copy evidence rather than a guess.

**And one dependency decision that came out *no* twice in one session**, beside the
three-hundred-and-ninety-second's. `tools/safedocs` reads HTTPS byte ranges and ZIP central
directories and takes **no package at all**: the transport is `curl` as a subprocess, so there is
no HTTP client, no TLS stack and no certificate store in this graph (the precedent is
`tools/pdfref` driving `pdftoppm`, `mutool` and `gs`), and the ZIP reading is 200 lines in tree
because every ZIP crate reads `Read + Seek`, which over HTTP means writing the range-issuing
adapter that is most of the work anyway. `flate2` already supplies the raw-deflate decoder *and*
the CRC-32 the verification needs, and `sha2` was already here for §7.6. `Cargo.lock` did not
move.

**Provenance is a principle-4 question**, and the tree has one precedent — `pdf-spec`'s Arlington
tables, built by `build.rs` from a pinned submodule. Vendored data arrives the same way: a
checked-in tool, a pinned upstream revision recorded beside the bytes, the licence file verbatim
next to what it covers.

## ISO 32000-1:2008, PDF/A-2's base document

The owner downloaded it on 2026-09-07 to `doc/PDF32000_2008.pdf`. Adobe publishes it without
charge — it is the edition ISO approved as ISO 32000-1:2008 — and the root `.gitignore`'s
`/doc/*.pdf` line already excludes it, which is the same treatment every other specification PDF
in this tree gets: **free to obtain is not free to redistribute**, and ADR 0187's discipline
applies unchanged.

**Why it matters here.** ISO 19005-2 §5.1 makes a conforming PDF/A-2 file one that adheres to all
of ISO 32000-1 as modified by that part, and every "ISO 32000-1:2008, 9.x" citation part 2 makes
points into it. Until now this tree carried only ISO 32000-2, so `crates/pdf-archive`'s `Part::Two`
records that a PDF/A-2 verdict leaning on a difference between the editions would be citing the
wrong one — `doc/questions/Q49` is the purchase that closes it, and this is that purchase, made for
nothing. Two things it immediately unparks are named where they act: Annex A's operator summary,
which three agents had recorded as a table nobody holds, and §9.7.5.2's predefined CMap table,
which ISO 32000-2 dropped and which PDF Association issue #77 is parked on.

**Both are prepared the same way.** `python3 tools/spec-md.py "doc/ISO_IEC 15444.pdf"` and
`python3 tools/spec-md.py doc/PDF32000_2008.pdf --out doc/md/ISO_32000-1_2008.md` put their text
under `doc/md/`, which is ignored. That script reads them with this tree's own `quorra-retrieve`
and writes into its header which order it got — the structure tree's where a document has one,
the page's content order where it has not — because the two are not equally trustworthy and a
reader quoting the result should know which they have. Nothing there is a gate's input:
`conformance::STANDARD` names exactly one file, ISO 32000-2's, so nothing can be mistaken for the
standard this project is written against.

**Its own permissions forbid extraction, and that is the reader's to set.** `pdfinfo` reports
`copy:no` on the file. `CLAUDE.md` principle 3's second half is explicit that a document's
restrictions are the reader's to switch off — "a restriction a reader cannot switch off is a
restriction imposed on the reader by somebody else's file, and this program is the reader's" — so
this tree's own reader opens it and answers questions about it, as it does for the SRPS reprints in
`doc/pdfa/`. That is a stance the project already took, not a new one taken here.

## ISO/IEC 15444-1:2000, JPEG 2000's core coding system

The owner obtained it on 2026-09-07 as `doc/ISO_IEC 15444.pdf` — *Information technology — JPEG
2000 image coding system — Part 1: Core coding system*, the 2000 first edition. `/doc/*.pdf`
already excludes it and the same licence position applies as to every specification here.

ISO 19005-2 §6.2.8.3's JPEG 2000 rules turn on fields no reader in this tree could see: Annex
I.5.3.3's `colr` box carries `METH`, `PREC`, `APPROX` and `EnumCS`, and Annex A.5.1's `SIZ`
marker segment carries the component count and each component's bit depth. Two agents had
declined those rules for exactly that reason, and correctly.

**What it does not carry, and that is now settled.** §6.2.8.3 also asks that only the JPX
baseline set of features be used, and that is defined in ISO/IEC 15444-**2**:2004 M.9.2.
`doc/questions/A51` decides it: part 2 will not be bought, and the row stays `Unchecked` with its
truthful reason rather than as a debt (`doc/adr/0928`).

**Two files beside it that look like later editions and are not.**
`doc/ISO-IEC-15444-1-2016.pdf` and `doc/ISO-IEC-15444-1-2019.pdf` are the third and fourth
editions' **iTeh STANDARD PREVIEW** documents: title page, copyright notice, foreword and table of
contents, fifteen pages each. Prepared with `tools/spec-md.py` they come to about 5 300 words and
contain the strings `colr` and `EnumCS` exactly zero times. They are recorded here so that a later
round reading the filenames does not spend an hour discovering it. Same licence position as
everything else here; `/doc/*.pdf` excludes them and `doc/md/` excludes their text.

**The edition trap is therefore permanent**, and it is recorded where it acts rather than only
here. ISO 19005 permits a `METH` of 3 where the 2000 edition defines only 1 and 2, names
enumerated colour spaces 12 and 19 where its Table I-10 defines only 16 and 17, and gives `APPROX`
a meaning where I.5.3.3 says the field shall be zero and readers shall ignore it. In all three the
rule implemented is ISO 19005's, and each constant in `crates/pdf-archive/src/table/graphics.rs`
says so above itself.

## ISO 15076-1:2010, the ICC profile format — a preview, and what it cannot answer

The owner downloaded it on 2026-09-10 as `doc/ISO-15076-1-2010.pdf` — *Image technology colour
management — Architecture, profile format and data structure — Part 1: Based on ICC.1:2010*.
`/doc/*.pdf` excludes it, `python3 tools/spec-md.py doc/ISO-15076-1-2010.pdf` put its text under
the ignored `doc/md/`, and the same licence position applies as to every specification here.

**It is an iTeh STANDARD PREVIEW and it reaches the introduction.** Title page, foreword, the
table of contents and clause 0's overview — and then it stops. Its own contents list puts clause 7
*Profile requirements* on page 18 and section 7.2 *Profile header* on page 19, and the preview
ends long before either. So the profile header is not in it, the `Profile ID` field is not in it,
and section 7.2.18's MD5 computation is not in it.

**What it does still answer is its own identity**, and that turned out to be the useful part. Its
foreword states that ISO 15076-1 is technically identical to ICC.1:2010 at profile version
4.3.0.0, and its introduction walks the chain before that: revision 4.2 was the first proposed as
an International Standard and became ISO 15076-1:2005. Its contents list also puts clause 7 at the
clause number ISO 19005-4 cites. Those are the two facts that let the ICC.1:2022 text below stand
in for section 7.2.18 — an identification, not a substitution.

**The cost of the preview is therefore smaller than it looked when this row was written.** The
three things it could not answer were the two `Unchecked` graphics rows and the corpus miss
`6-2-4-2-t03-fail-e`; the miss closed in the nine-hundred-and-fiftieth session on ICC.1:2022's and
ICC.2:2023's statements of the same clause, and the rows were split rather than left whole. What
this preview still cannot give is what a profile stating version 4.3.0.0 has to satisfy — every
requirement of clause 7 onward — which is why a profile naming that edition is judged by nothing
here.

## ICC.1:2022, the ICC profile format — the whole text

The owner downloaded it on 2026-09-10 as `doc/ICC.1-2022-05.pdf` — *Image technology colour
management — Architecture, profile format, and data structure*, the International Color
Consortium's own edition, profile version 4.4.0.0. `/doc/*.pdf` excludes it,
`python3 tools/spec-md.py doc/ICC.1-2022-05.pdf` put its text under the ignored `doc/md/`, and the
same licence position applies as to every specification here.

**It is complete**: 126 pages, about 54 000 words, clause 1 through Annex G — the profile header
field by field, the tag table, every tag and every tag type, and the required-tag lists of clause
8. Not a preview.

**Its redistribution terms are not established.** The ICC's specifications are free to obtain and
that is not the same as free to redistribute, which is ADR 0187's distinction and the reason
`doc/md/` is ignored. Nothing was found stating what may be republished, so it is treated exactly
as `doc/pdfa/`'s two files are — **licensed to a single reader** — and **nothing in this tree
quotes it**: cite and paraphrase, in the code as much as in the documents. Any code comment
reproducing a sentence of it would be the thing this position exists to prevent.

**What it answered**, in the nine-hundred-and-fiftieth session:

- **Section 7.2.18, the `Profile ID` field.** Bytes 84 to 99, holding RFC 1321's MD5 over the
  whole profile — as long as the header's size field says — with the profile flags, the rendering
  intent and the field itself zeroed for the calculation, and a zero field meaning no identifier
  has been calculated. That is `pdf_model::icc::computed_id`, and it closed the corpus miss
  `6-2-4-2-t03-fail-e`, whose two profiles differ in exactly those sixteen bytes.
- **Section 7.2.4, the profile version field.** Binary-coded decimal, byte 8 the major version and
  byte 9 the minor and bug-fix versions in its two halves, the numbers set by the ICC, and 4.4.0.0
  stated as the version consistent with this edition. That is what makes a version number identify
  an *edition*, which is the premise
  `crates/pdf-archive/src/table/graphics.rs`'s `IccEdition` rests on.
- **Clause 8's required tags**, for the display and output classes a PDF/A destination profile may
  be.
- **That it stands in for ISO 15076-1:2010 on section 7.2.18, and why that is an identification
  rather than an assumption.** Its foreword says it is an update to ICC.1:2010, that ICC.1:2010 and
  ISO 15076-1:2010 are technically identical, and lists the technical changes it makes — none of
  which touches the profile header or that calculation. ICC.2:2023 states the same method under its
  own clause number, so two held texts agree.

**What it does not answer.** ISO 19005-2 section 6.2.4.2 admits ICC.1:1998-09, ICC.1:2001-12,
ICC.1:2003-09 and ISO 15076-1, and ICC.1:2022 is none of the four: it is a fifth text at a version
later than any of them. Two of the four arrived separately and are recorded below; ICC.1:2003-09 is
not obtainable — the ICC supplies its past specifications on request only — and ISO 15076-1:2010 is
the preview above.

**And the ICC publishes amendments and errata to its own specifications**, which is worth a line
here because this project already treats approved errata as an input for ISO 19005
(`doc/errata-read.md`). The ICC's v4 page links two amendments to ICC.1:2022 — an Adaptive Gain
Curve tag and a CICP tag amendment — and lists errata for ICC.1:2010-12 and ICC.1:2004-10 without
links. All of them amend texts at version 4.4 or later, which ISO 19005-2 does not name, so none
bears on a row here today. A round that starts judging 4.3.0.0 or 4.4.0.0 profiles in earnest owes
a look at them first.

## ICC.2:2023, iccMAX — the whole text

The owner downloaded it on 2026-09-10 as `doc/ICC.2-2023.pdf` — *Image technology colour
management — Extensions to architecture, profile format, and data structure*, profile version
5.0.0.0. Same treatment and the same unestablished terms as ICC.1:2022 above: `/doc/*.pdf`
excludes it, its text is under the ignored `doc/md/`, it is treated as licensed to a single
reader, and nothing in this tree quotes it.

**It is complete**: 255 pages, about 90 000 words.

**What it answered**, in the same session — and it answered the question this project had been
unable to ask at all:

- **A profile stating version 5.0.0.0 claims iccMAX**, by its section 7.2.6, and its section 1
  places iccMAX beside ISO 15076-1 rather than inside it — a document based on ISO 15076-1 that
  expands the profile specification, removing some of its types and adding others. So such a
  profile does not claim any of ISO 19005-2's four texts, which is
  `graphics/icc-profiles-claim-a-permitted-edition`.
- **The corpus witness `6-2-3-t01-fail-d` is decided, and against the file.** It is an Adobe RGB
  (1998) profile whose version field has been set to 5.0.0.0: a display profile carrying the v2
  matrix and curve tags and nothing else. ICC.2 section 8.4 requires a display profile to carry one
  or more of `AToB0Tag` to `AToB3Tag` or `DToB0Tag` to `DToB3Tag` and one or more of `BToA0Tag` to
  `BToA3Tag` or `BToD0Tag` to `BToD3Tag`; it carries none of the sixteen, and iccMAX defines no
  matrix-column or TRC tags at all. Its `Profile ID` field states a value that is not the one
  section 7.2.20's method gives either. Both are now reported, and the miss is closed under both
  parts.
- **Section 7.2.20 is section 7.2.18 again**, word for word on the calculation, which is the second
  held witness for the method ISO 19005-4 cites.

**What it does not answer.** Everything about a profile stating any other version — and, for a
version 5 profile, everything this project has not read of its clauses 9 and 10.

## ICC.1:1998-09 and ICC.1:2001-12 — two of the four texts ISO 19005-2 names

The owner downloaded both on 2026-09-10, as `doc/md/ICC-1_1998-09.md` (*File Format for Color
Profiles*, profile version 2.2.0, 134 pages, about 56 000 words) and `doc/md/icc_1_2001-12.md`
(*File Format for Color Profiles (Version 4.0.0)*, 126 pages, about 62 000 words). Both are
complete texts rather than previews. Same treatment and the same unestablished terms as the two
editions above: their PDFs and their markdown are ignored, they are treated as licensed to a single
reader, and nothing in this tree quotes either.

**They are the first two of ISO 19005-2 section 6.2.4.2's four**, which is what makes them
different from ICC.1:2022 and ICC.2:2023. That clause's list now stands at ICC.1:1998-09 **held**,
ICC.1:2001-12 **held**, ICC.1:2003-09 **not held**, ISO 15076-1 **preview only**.

**The second file is prepared in a mixture of both page orders**, which its own header records. It
is readable clause by clause all the same; a round quoting a line number from it should check the
surrounding page markers rather than assume the file runs forwards.

**What they answered**, in the nine-hundred-and-fiftieth session:

- **Clause 6.3 of each, the required tags per profile class and form.** Both begin every
  device-class table from `profileDescriptionTag`, `mediaWhitePointTag` and `copyrightTag` and then
  add the transform the form is built on. That is
  `graphics/icc-profiles-carry-the-tags-a-permitted-edition-requires`, which judges every
  `ICCBased` profile against both — a profile conforming to neither conforms to neither of the two
  texts of the four this project can read.
- **The version numbering, from the earliest end.** ICC.1:1998-09 section 6.1.3 states its own
  number as 2.2.0 and says a major version change happens only for an incompatible change — new
  required tags is its example — while a minor one carries compatible changes such as new optional
  tags. Its Annex F then lists what each earlier revision changed. Together those say that nothing
  *required* was added inside major version 2.
- **That the shipped `data/icc/sRGB2014.icc` is admissible under ISO 19005-2 after all**, which was
  an open question worth asking: the profile's header states 2.0.0, and ICC.1:1998-09 states its own
  number as 2.2.0. Neither that text nor part 2 requires a profile to *state* the number of the
  edition it conforms to — part 2's sentence is a disjunction over documents — and the profile
  carries exactly the nine tags ICC.1:1998-09's Table 27 requires of an RGB display profile. Its
  `Profile ID` at bytes 84 to 99 offends nothing either: that text's Table 9 gives bytes 84 to 127
  to reserved expansion and, unlike its other reserved fields, states no requirement that they be
  zero. `doc/adr/0950` records the reading and `data/icc/PROVENANCE.md` stands.
- **That the `Profile ID` calculation is not edition-invariant.** ICC.1:2001-12 section 6.1.13 puts
  the field at the same bytes and zeroes a *different* set of header fields for the digest — the
  rendering intent, the device attributes and the field itself, where ICC.1:2022 and ICC.2:2023
  zero the profile flags in the attributes' place — and it does not state the method at all, but
  points at a technical note on the ICC's web site. So `pdf_model::icc::computed_id` is the later
  method, which is the one ISO 19005-4 names, and `pdf_archive` does not apply it to a profile
  claiming 4.0.0.

**What they do not answer.** Everything about a profile claiming a version between them —
2.1.0 above all, which is most of the profiles that exist — since neither text is the edition such
a profile names, and everything of either document beyond clause 6.3's tag lists.

## The XMP Specification, read for a table of facts

ISO 19005-2 §6.6.2.3.1 requires every XMP property to come from a predefined schema, and deciding
that needs to know what those schemas *contain* — the corpus's 273 witnesses for the clause are
almost all a predefined property carrying the wrong value type, not an unknown property. The
standard names its source in its own bibliography, entry [20]: *XMP: Extensible Metadata Platform,
September 2005, Adobe Systems*. The AIIM address it gives is dead; the document was read from the
Internet Archive, and the current edition (XMP Specification Part 2, *Additional Properties*, 2017)
beside it.

**What was taken is a table of facts, not text.** `crates/pdf-archive/src/table/metadata.rs`'s
`PREDEFINED` carries 274 property names across 14 schemas, each reduced to the two things a reader
can check — a shape (simple, language alternative, alternative, seq, bag, structure) and a lexical
form (Boolean, Integer, Real, Rational, Date, GPS coordinate, or none). That is the same kind of
taking as the standard-14 metrics in `crates/pdf-font/src/standard_metrics.rs`: a list of values a
specification states, not a passage of its prose. **Nothing is quoted**, and the tables are treated
with the caution `doc/pdfa/`'s two files get.

Two readings came out of it and both are recorded where they act:

- **`xmp:Rating` is a Real.** The 2005 edition types it *Closed Choice of Integer* and the 2017
  edition *Closed Choice of Real*. §6.6.2.3.1 cites "the XMP Specification" **undated**, and
  clause 2's own rule for an undated reference is that the latest edition applies — so the later
  typing governs. This is a fact about the specification, arrived at from its own citation rule.
- **A property whose local name the table does not carry is not judged.** Later editions add
  properties — `photoshop` 14 to 20, `xmpDM` 57 to 66 between 2005 and 2017 — so refusing an
  unknown name would fail conforming files over nothing but the age of the transcription.

## ISO 16684-1:2012, the XMP specification — a preview that reaches section 7.2

The owner downloaded it on 2026-09-10 as `doc/ISO-16684-1-2012.pdf` — *Graphic technology —
Extensible metadata platform (XMP) specification — Part 1: Data model, serialization and core
properties*. `/doc/*.pdf` excludes it, `python3 tools/spec-md.py doc/ISO-16684-1-2012.pdf` put its
text under the ignored `doc/md/`, and the same licence position applies as to every specification
here. It is licensed to a single reader like `doc/pdfa/`'s two files, so nothing in this tree
quotes it: cite and paraphrase.

**It is an iTeh STANDARD PREVIEW too, and this one carries real clause text.** Clause 1 through
section 7.2 — the scope, the normative references, the terms, the notations, clause 5's
conformance, the whole of clause 6's data model, and the general and equivalent-forms preamble of
clause 7's serialisation. It stops there. Sections 7.4 to 7.8, which are the canonical RDF
serialisation, and section 7.9, which says which equivalent RDF forms are allowed and prohibited,
are **not** in it; neither is clause 8's core properties, whose tables this tree reads from Adobe's
own edition instead (the section above).

**What it settled**, in the nine-hundred-and-forty-sixth session:

- ISO 19005-2 section 6.6.2.1's and ISO 19005-4 section 6.7.2.1's one `Unchecked` serialisation
  row became five rows — two checks over the data model and the single `rdf:RDF` element, two
  naming held sentences `pdf_model::xmp` cannot see, and one keeping what the preview does not
  reach. `crates/pdf-archive/src/table/metadata.rs`'s module comment has the split and the reason
  the data-model row binds part 2 alone.
- **The encoding of an XMP packet is nobody's rule.** Section 7.1 names UTF-8, UTF-16 and UTF-32,
  puts the choice between them beyond its own scope, and leaves it to whichever standard embeds
  the packet — and neither ISO 19005 part nor ISO 32000-2 §14.3.2 states one. That closed the
  corpus miss `6-7-2-1-t01-fail-e`, whose expectation is that a packet not encoded as UTF-8
  fails; `crates/pdf-archive/tests/corpus.rs` carries the ruling and names the three documents
  read.
- **A comparison of `xml:lang` values is case-insensitive** (section 6.4, by way of IETF RFC
  3066), and `pdf_model::xmp` compared them exactly — so a packet writing `X-Default` had no
  title this reader could find. Fixed where it acts.
- A citation was wrong and is corrected: `pdf_model::xmp` attributed the three permitted encodings
  to a section 7.3.2, and the sentence is section 7.1's, saying rather less than the citation
  implied.

**What it cannot answer.** Which RDF spellings a packet may use and which it may not — that is
sections 7.4 to 7.9 — so `metadata/xmp-packets-meet-the-xmp-serialisation` stays `Unchecked` with
that as its reason. A round wanting it needs the full text; the preview will not grow.

## PDF Association TechNote 0010, and a licence that could not be confirmed

The owner obtained it on 2026-09-09 as `doc/TechNote0010.pdf` — *TechNote 0010: Clarifications of
ISO 19005, parts 1-3 for developers of PDF/A creators and validators*, PDF Association, 2017,
twenty-eight items each carrying a resolution of the ISO working group responsible for ISO 19005.
`python3 tools/spec-md.py doc/TechNote0010.pdf` put its text under `doc/md/`, read with this
project's own `quorra-retrieve`, and `/doc/*.pdf` and `doc/md/` are both ignored — the same
treatment every specification here gets. It is what session 941 acted on; `doc/adr/0931` says what
this tree does with a clarification that is neither the standard nor an erratum, `doc/adr/0933`
what session 942 found on taking eight more of its items, and
`crates/pdf-archive/src/clarification.rs` is where every item taken from it lives, beside the ones
recorded as confirming a reading, as owed, or as bearing on no row at all.

| data | source examined | terms |
|---|---|---|
| PDF Association TechNote 0010 (2017) | the document itself, page 1 and its XMP packet | **stated copyright, no stated grant** — see below |

**What the document says about itself is one line.** Page 1 carries a copyright notice naming the
year and nothing else: no licence, no permission, no terms of use. Its XMP packet carries no
`dc:rights` and no rights-management properties at all — which is worth recording precisely
because the file is a conforming PDF/A-2a document whose producer filled in everything else.

**What the publisher says could not be reached.** `pdfa.org` returns HTTP 403 to this machine, as
it did in sessions 939 and 940 and as it does to both a browser user agent and a plain fetch, so
the resource page's own terms are unread. `doc/rfc/0006` records the document as CC-BY-4.0, and a
web search agrees, but **neither is a licence text this project has seen** — a second-hand report
of a licence is the same kind of evidence as a second-hand report of a clause, and principle 5's
discipline does not change subject just because the subject is copyright.

**So the terms are treated as unclear, and the consequence is a rule rather than a worry: nothing
in this tree quotes it.** Every one of its resolutions is cited by item number and paraphrased in
this project's own words — `TechNote 0010 section A021`, never a sentence of it between quotation
marks — which is exactly the discipline `doc/pdfa/`'s ISO reprints already impose and which costs
nothing here, because what a validator needs from the note is the rule and not the wording. If the
publisher's page becomes reachable and states CC-BY-4.0, the row above can be tightened to a
licence and the quotation rule relaxed; until then it stays as written.

**One further caution, and it is about reading rather than copyright.** Each item prints the case
and then the verdict, and they are not the same thing: a problem statement, sometimes a *PDF
Validation TWG proposal*, and then an *ISO WG Resolution*. Only the last is acted on here, and
`crates/pdf-archive/src/clarification.rs` says why — at four items the working group left the
published requirement as it stood, so a project that read the cases rather than the verdicts would
have changed four rules the committee did not change.

## ISO/WD 8601-1 (2016), the dating standard's working draft

The owner downloaded it on 2026-09-12 as `doc/iso-tc154-wg5_n0038_iso_wd_8601-1_2016-02-16.pdf` —
ISO/TC 154/WG 5 N0038, *ISO/WD 8601-1, Data elements and interchange formats — Information
interchange — Representation of dates and times — Part 1: Basic rules*, dated 2016-02-16 — in answer
to `doc/questions/Q53`, which asked whether to obtain ISO 8601 so that `Lexical::Date` could admit
the standard's forms rather than the XMP Specification's six profiles of them. `/doc/*.pdf` ignores
it, and `crates/pdf-archive/src/iso_8601.rs` is what was read from it, cited to its clause numbers.

| data | source examined | terms |
|---|---|---|
| ISO/WD 8601-1, ISO/TC 154/WG 5 N0038 (2016-02-16) | the document itself, its cover and copyright page | **a working draft, copyright ISO**: reproduction permitted to participants in the ISO standards process and to nobody else; neither the document nor any extract is to be reproduced otherwise without ISO's written permission — see below |

**Two things its cover says that bind how this tree uses it.** First, it is not an International
Standard — its own warning says it is distributed for review and comment, may change without
notice, and may not be referred to as one. So every citation in this tree names the working draft
by its document number and clause, never "ISO 8601-1:2016", and `doc/questions/A53` records that
the published part is the purchase to make on the day a difference between the two matters.
Second, its copyright notice grants reproduction to participants in the standards process only,
which this project is not; so **nothing in this tree quotes it** — the grammar in `iso_8601.rs` is
this crate's own sentences under the draft's clause numbers, the same discipline `doc/pdfa/`'s
reprints and `TechNote 0010` already impose, and the draft's own example values (`19850412`,
`1985-W15-5`) appear in tests as data rather than as prose.

## veraPDF, and the difference between running a program and reading it

The owner put a checkout of veraPDF in `doc/veraPDF-library` on 2026-09-07 and asked whether its
licence lets this project look at the code. It is dual-licensed **GPLv3+ or MPLv2+** — both
licence texts are in the checkout, as `LICENSE.GPL` and `LICENSE.MPL`, and `LICENSE-HEADERS.md`
says the dual licensing is deliberate and applies to all veraPDF software. `doc/.gitignore` line 2
excludes the directory, which is the same treatment `/doc/rasterrocket` gets and for the same
reason: a checkout kept beside the tree for comparison, never vendored into it.

**Three uses, and they are not alike.**

1. **Running it is unrestricted.** Neither licence limits *use*; both are about distribution. So
   running veraPDF over a corpus and comparing its verdicts with `pdf-archive`'s costs nothing and
   risks nothing. **This is the use worth having**, and `CLAUDE.md` principle 5 already says
   exactly what it is worth: agreement raises confidence that we read ISO 19005 correctly, and
   disagreement is a question to take back to the standard. It is the same relationship this tree
   has with poppler, mupdf and pdf.js in `tools/pdfref`.

2. **Taking code is out**, and not marginally. This project is Apache-2.0; `deny.toml`'s allow-list
   carries no GPL and no MPL, and the sentence at the top of this file — that nothing in this tree
   is under either — is checked by `cargo deny check licenses`. Copying a method, or writing one
   from an open editor, would make that sentence false.

3. **Reading it is the interesting case, and the answer is that we have no reason to.** Copyright
   restricts copying expression, not learning facts, so reading is not itself a licence problem;
   what it creates is a *question* about whether what you write afterwards is derived, and that
   question is worth nothing to us because **veraPDF's rule set is not a source of truth here**.
   RFC 0006 §10 question 2 put it before the owner in those terms — a conformance verdict derived
   from veraPDF's profiles is somebody else's reading of a text — and the situation has since
   improved rather than changed: `doc/pdfa/` now holds ISO 19005-2 and -4 themselves, which is a
   better source than any implementation of them. So the rule for this tree is the one principle 5
   already implies:

   > **Run veraPDF; do not read it.** Where it disagrees with `pdf-archive`, read the clause it
   > cites in `doc/pdfa/`, not the code that cites it.

   That rule is cheap to keep and it removes the derivation question entirely, which is the
   pleasant part: the licence-safe path and the principled path are the same path.

**Its test files are a separate question, and the answer about this checkout is that it has
none.** Counted rather than assumed: `find doc/veraPDF-library -iname '*.pdf'` returns **zero**.
What `core/src/test/resources` holds is 122 XML fragments (XMP packets and policy reports), four
text files and a handful of schemas — the inputs to unit tests of the XMP and policy readers, not
documents. The validation profiles are not here either; the checkout carries
`validationProfile.xsd`, the schema they are written against, and nothing written in it. veraPDF
splits those across repositories, so a corpus and a profile set are each a separate fetch with a
separate licence to read.

**The corpus was fetched the same day, and its licence is the good outcome.** It is a *separate
repository* from the library and carries a *different* licence: `doc/veraPDF-corpus/README.md`
states **Creative Commons Attribution 4.0 International (CC BY 4.0)** — permissive, no copyleft,
redistribution allowed with attribution. So none of the GPL/MPL question above touches it, and it
is a **submodule** under `doc/veraPDF-corpus` rather than an ignored directory: the four corpora
under `doc/corpora/` are tracked the same way, and a submodule carries no bytes into this history.

What it holds, counted rather than assumed: **2 908 documents**, in directories named by clause
(`PDF_A-4/6.9 Embedded files`) with the intended verdict in each file name
(`…6-9-t02-fail-a.pdf`, `…6-9-t03-pass-a.pdf`). Its README says that pattern is deliberate — the
files are atomic, self-documented through their outlines, and named for the clause they exercise.
`crates/pdf-archive/tests/corpus.rs` reads both facts off the path rather than maintaining a table
beside them.

**Principle 5 still governs what it is worth.** A corpus file's intended verdict is its author's
reading of ISO 19005, not the standard's text — so the harness *reports* where the two readings
differ and does not gate on them, and each disagreement is a question for `doc/pdfa/` with three
possible answers: this crate is wrong, the corpus is wrong, or the clause is ambiguous. It is a
**robustness** population and a second opinion. The **coverage** population is still ours to write,
one fixture per requirement row, because only there is the expected value derivable from a clause
we have read.

**One obligation to remember if anything is ever redistributed.** CC BY 4.0 requires attribution.
A submodule distributes nothing, so nothing is owed today; a round that copies a corpus file into
this tree as a fixture, or ships verdict data derived from it, owes the attribution and adds it to
`/NOTICE` beside the font rows.

**The population principle 5 actually wants, we can write.** A requirement table is a list of
rules, so a fixture per row, built to break exactly that row, has an expected verdict derivable
from a clause in `doc/pdfa/` — which is what `CLAUDE.md` asks of every test's expected value and
what `crates/pdf-archive/tests/verdict.rs` already does for the first tranche. Our own fixtures are
the better *evidence*; somebody else's corpus is the better *coverage*. The two answer the two
questions `CLAUDE.md` keeps apart, and neither substitutes for the other.

**Nothing was taken and nothing was read.** This entry is the record that the question was asked
and answered, in the same form as the `cidToUnicode` row above: examined, and not taken.
