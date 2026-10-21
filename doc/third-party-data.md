# Third-party data and dependency decisions

Status: **record** — what was read, off copies on this machine, and what each decision cost.
Read by: whoever is about to vendor data or take a dependency. The licence obligations this
lists are met by `/NOTICE` and checked by `viewer-ui/tests/notices.rs`.

`doc/HANDOVER.md`'s reader table points a round taking a dependency here (ADR 0133's "`HANDOVER.md` §1" means this file).

**This project is Apache-2.0**, on the project owner's word, with one author in the whole
history, so no relicensing has needed anybody else's consent. Every package in the graph is under a
licence on `deny.toml`'s `[licenses] allow` list, each permissive and each combinable into an
Apache-2.0 work; nothing in this tree is under GPL or MPL — the one GPL item this record names is
poppler's `cidToUnicode` data in the table below, examined and **not** taken. `cargo deny check
licenses` is the command that keeps that true.

**Apache-2.0 section 4's obligation is met by a file that already existed**: a redistribution
carries the licence text and, where the work has one, its NOTICE file. `/LICENSE` and `/NOTICE` are both at the root and CI packages both beside every binary,
which is what the BSD-3-Clause font programs already needed.

**The data this tree vendors**, and what was read for each — off copies on this machine, not
recalled:

| data | source examined | terms |
|---|---|---|
| Adobe predefined `CMap`s | `poppler-data`'s `COPYING.adobe` (1990–2019), `doc/pdf.js/external/bcmaps/LICENSE` (2009), `hayro-cmap`'s `assets/LICENSE.txt` (2023) | **BSD-3-Clause** |
| Foxit standard-14 programs | `doc/pdf.js/external/standard_fonts/LICENSE_FOXIT`, from PDFium | **BSD-3-Clause** |
| Liberation Sans | `LICENSE_LIBERATION` | **SIL OFL 1.1** (reserved font name: ship and use freely, do not modify and keep the name) |
| poppler's `cidToUnicode`, `nameToUnicode`, `unicodeMap` | `poppler-data`'s `COPYING` | **GPL-2 or GPL-3** — Glyph & Cog's, *not* Adobe's |

Archive output follows owner answer A550 (ADRs 2194–2195): newly embedded or
rewritten bundled Foxit font programs carry the complete notice in XMP UsageTerms. Modified
Liberation programs rename their active reserved identities to QuorraText;
unmodified programs and copyright/license records are preserved.

`BSD-3-Clause` costs three obligations: reproduce the notice and disclaimer "in the documentation
and/or other materials provided with the distribution", keep them in source, never use Adobe's or
Google's name to endorse this. **The surface for that is `/NOTICE`** — at the repository root,
`include_str!`d and printed by `quorra --licences`, put over the page by `?` as the About panel, set in
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

**What shipped**: `data/standard-fonts/` holds §9.6.2.2's fourteen font programs, so those pages
reproduce on any machine (ADR 0133); `data/cmaps/` holds every `CMap` Adobe publishes, deflated one
at a time by `build.rs` into a blob and inflated only when a document names one, so nothing is
decompressed at startup (ADR 0140), and §9.10.2's third method came with it.

**Three things worth keeping out of the first**: PDFium's `.pfb` files are bare CFF programs
(`01 00 04 02`), a name-keyed substitute is addressed by glyph *name* and so needs no Adobe Glyph
List step — which is why `Symbol` and `ZapfDingbats` work — and a *composite* font cannot use them
at all, because §9.7.4.2 leaves it reachable only through `/ToUnicode`, which addresses by
character.

**What none of it fixes** is a font naming an `Identity` ordering, where the codes index a font
nobody supplied: [todo 21](todo/21-font-substitution.md). **A todo file's claim decays as a ledger
row's does**: that file once named two documents as undrawable that ADR 0153's coverage rule had
already fixed, and `doc/todo/01`'s sweeps read `ledger.toml` and `crates/`, not `doc/todo/`'s claims.

**`accesskit` 0.24.1 and `accesskit_unix` 0.22.1** (ADR 0214), both **MIT OR Apache-2.0**, every
package behind them permissive; the three unmaintained `zbus` 5 crates they reach are ignored by
identifier in `deny.toml` (ADR 2038, `doc/stack.md`). Two things about it belong here: **`memchr` is not among them at runtime** — it arrives through `winnow` and
`proc-macro-crate`, which are a proc macro's build dependencies, so the rule ADR 0186 refused
`quick-xml` over is intact and `cargo tree -e normal` is how that was checked; and **the one async
runtime in this tree is `accesskit_unix`'s**, which lives on a thread of its own, is named by one
crate, is `cfg(target_os = "linux")` in that crate's manifest, and is not created until the first
frame has been presented.

**`sha1` 0.11.0 and `ripemd` 0.2.0** (ADR 0215), **MIT OR Apache-2.0**, from the RustCrypto family of
the `sha2` and `md-5` §7.6 brought in, with no transitive dependency `sha2` did not already pull.
They exist because §12.8.3's Table 260 and Table 256 name digest algorithms the others do not cover,
and implementing five while silent about the sixth is the failure this project removes.

**A dependency decision that came out `no`**: §12.8.1's *second* question needs an X.509
certificate reader and an RSA verification, and `rsa` and `x509-cert` are declined — `rsa` 0.9 brings
a second `digest` 0.10 hash stack beside this tree's 0.11, and `x509-cert`'s strict DER would refuse
the corpus signatures that state indefinite lengths — so `pdf_signature::x509` and
`pdf_signature::pkcs1` are in tree under `#![forbid(unsafe_code)]` (ADR 0229). The elliptic curves
ADR 0229 declined with them are taken (ADR 0532, superseding the refusals of ADRs 0314 and 0331),
and the arithmetic is `crypto-bigint`'s (ADR 0331); `doc/stack.md` is the current position.

**`gtk4` 0.11.4, a *toolkit*** (ADR 0244): the GNOME project's own GIR-generated binding, with
`glib`, `gio`, `gdk4`, `gsk4`, `pango`, `cairo-rs`, `graphene-rs`, their `-sys` crates and the
`system-deps`/`pkg-config` machinery behind it, every one MIT. Three things about it belong here. **It is confined to one crate**: `gtk4`
is named by `crates/viewer-gtk`'s manifest and by no other, nothing that touches PDF bytes gains a
toolkit dependency, and `viewer-ui`'s and `viewer-confined`'s graphs are byte-for-byte what they
were. **It costs no `unsafe` in this tree**: `viewer-gtk` and its binary both keep
`#![forbid(unsafe_code)]`, because `gtk4-rs` is a safe binding and the `unsafe` is inside
`gtk4-sys` and `glib` — which is the property `doc/todo/30` chose GTK4 for and the property the Qt
host will not have. And **it is linked against the platform's own libraries** rather than vendored:
a native host depending on the desktop being installed is the point of a native host, and it is why
`viewer-gtk` is in none of `doc/verify.md`'s cross-target checks, by the same rule that makes
`viewer-accessibility` Linux-only in its own manifest — `glib-sys`'s build script wants a
cross-compiling `pkg-config` and there would be no GTK 4 development files for the target anyway.

**`cxx` 1.0.199 and `cxx-qt-build` 0.9.1, a *C++ bridge*** (ADR 0246): of what arrives, only
`cxx`, `cxxbridge-macro` and `link-cplusplus` reach a shipped binary — the rest (`cxx-qt-build`,
`cxx-gen`, `qt-build-utils`, `clang-format`, …) are build-time only, which `cargo tree -p viewer-qt
-e normal` shows — and every package is MIT, Apache-2.0 or Unlicense-or-MIT. Four things about it
belong here. **It is confined to one crate**: `cxx`
and `cxx-qt-build` are named by `crates/viewer-qt`'s manifest and by no other, nothing that touches
PDF bytes gains either, and `viewer-ui`'s, `viewer-gtk`'s and `viewer-confined`'s graphs are what
they were. **`cxx-qt` itself was declined**, which is the part worth recording as a decision: the
crate that makes a Rust type into a `QObject` is built for QML and links two initialisers a Widgets
host never calls, and only the *build* half — finding Qt through `qmake6`, running `moc`, linking
QtCore/QtGui/QtWidgets — is wanted, and it is available on its own. **It costs one hand-written
`unsafe` token**, the `unsafe extern "C++"` block header, which is `cxx`'s way of asking the author
to assert that the C++ declared there exists and is safe to call; `viewer-qt` holds
`#![deny(unsafe_code)]` with one exemption on `mod bridge`, and `tests/unsafe_position.rs` asserts
the file and the token and names every crate that lifts the denial — `viewer-ffi`, `pdf-vfs-ffi` and
the three workers' programs besides this one, each with a position test of its own (ADRs 0247, 0868,
1945). `viewer-ffi` took **no** dependency: its header is hand-written, so there is no `cbindgen`. **And it is linked
against the platform's own Qt** rather than vendored, for the reason `viewer-gtk` links against the
platform's GTK: that is what a native host is, and it is why `viewer-qt` is in none of the
cross-target checks — `cc-rs` wants `lib.exe` for a Windows target and there would be no Qt 6
development files there anyway.

**The corpora** (ADR 0258): a submodule carries no bytes into this history, so the licence
question is what may be *promoted* out of one rather than what may be shipped. The positions, each
read off the repository's own licence file rather than off GitHub's guess:

| submodule | licence, as examined | what it permits here |
|---|---|---|
| `doc/corpora/pdf20examples` | `LICENSE.md`: "The PDF Association provides these example PDF 2.0 files under the Creative Commons Attribution-ShareAlike 4.0 International (CC BY-SA 4.0) license." | submodule freely; a file **copied** into this tree would carry attribution and the licence with it, and CC BY-SA 4.0's ShareAlike condition attaches to *Adapted Material* rather than to a Collection, so a verbatim copy beside this tree's Apache-2.0 code does not relicense the code |
| `doc/corpora/pdf-differences` | `LICENSE`: the Apache License 2.0 in full — **and the root `README.md` splits the repository in two**: "PDF files are copyright by the PDF Association and distributed under a [Creative Common Attribution 4.0 International (CC BY 4.0) license](https://creativecommons.org/licenses/by/4.0/). Any source code in this repository is licensed under [Apache License, Version 2.0]" | submodule and copy freely; a **PDF** copied out carries CC BY 4.0's attribution rather than Apache-2.0's notice, and the `README.md` beside each test case is where the attribution comes from |
| `doc/corpora/pdfbox` | `LICENSE.txt` Apache-2.0, with `NOTICE.txt` beside it | as above, and `NOTICE.txt` is the file §4(d) obliges anyone redistributing to carry — which is why the sparse checkout keeps the repository root rather than the test directory alone |

**Nothing is copied**, so none of those obligations is live: a witness is named by its path inside
a pinned submodule, and the only `.pdf` files under `crates/` (`find crates -name '*.pdf'`) are
fixtures this tree builds itself (ADR 1059). That is the cheapest answer to a licence question, and
it is the rule rather than an accident.

**`openpreserve/format-corpus`** carries no licence GitHub detects; its `README.md` says "[a]ll
items are CC0 licenced **unless otherwise stated**", and per-file `.md` sidecars are where such a
statement would be — a grant with an escape clause is not one this project relies on without
reading it (ADR 0187). Every sidecar was read (`doc/oracle-and-corpus.md` §2c) and the question put
to the project owner, **who reversed the caution rather than answering the file**:

> Add as many submodules as you want unless we can clearly deduce from their licence that we are
> not allowed to do so. We don't even republish, so most licence notes don't even apply to us!
> (We should still mention them as a courtesy.)

That is a rule rather than a ruling, and the fourth corpus is taken under its two halves. **A submodule is a pin, not a copy**: this repository records a URL and a commit
identifier, so a clone fetches those bytes from *their* server under *their* terms and this
history carries none of them — which is why the redistribution conditions that dominate the table
above are not engaged at all. What remains is the courtesy, and it is met here and in `/NOTICE`
§3. ADR 0305.

**So `doc/corpora/format-corpus` is the fourth**, pinned at `366f068c`; `doc/oracle-and-corpus.md`
§2b's recipe checks out three of its PDF directories sparsely (the owner's checkout on this machine
is whole, `git -C doc/corpora/format-corpus sparse-checkout list` says so). Each row is what that
directory's own sidecar says, quoted, and where it says nothing this table says so rather than
implying terms it does not carry:

| directory | what its own files state | why it is here |
|---|---|---|
| `pdf-handbuilt-test-corpus` | **nothing of its own**, so the repository's root default applies; its `README.md` credits an iPRES 2017 paper and a deposited artefact, DOI [10.22000/53](https://dx.doi.org/10.22000/53) | the reason for the whole exercise: files carrying **one** deliberate structural defect apiece, all drawing the same *Hello PDF-world!*, so a blank page is a finding without a reference (ADRs 0302, 0303, 0305) |
| `pdfCabinetOfHorrors` | **CC0, explicitly**: its `readme.md` ends "All files in this folder: Creative Commons CC0: Public Domain Dedication." | archival horrors — encryption at three permission settings, embedded video, a corrupt byte, a JPEG whose `/Height` was altered |
| `govdocs1-error-pdfs` | **otherwise stated, and permissively**: "All PDF files in this folder and subfolders are copied from Govdocs1", quoting Govdocs1's own "freely available for research and may be (to the best of our knowledge) freely redistributed" and asking for a citation — Garfinkel, Farrell, Roussev and Dinolt, *Bringing Science to Digital Forensics with Standardized Forensic Corpora*, DFRWS 2009, which this line is | `.gov` documents from the 1990s and 2000s that broke somebody else's software: legacy producers, four unparsable CFF programs and a truncated `head` table between them |

**Two directories are left out of the recipe, and neither for a licence that forbids** — the
owner's rule turns on *clearly forbids* and neither does:

- **`jhove-errors`** — no sidecar at all, and its files are published journal
  articles and theses from Springer, Wiley and university repositories. The root CC0 default is
  unreliable rather than absent here: a third party is in no position to dedicate somebody else's
  paper to the public domain. That is an *absent grant* rather than a prohibition, so under the
  owner's rule it could be pinned; it is left out on size alone (`du -sh` it), since it would
  multiply what this corpus costs a fresh clone. It is not without value — one of its files is a
  paper this tree showed no page of, on a `startxref` eight megabytes from the end of the file (ADR
  0379), guarded by a hand-built pair — and a round wanting the population reads it from a
  whole checkout or fetches it into `corpus-cache/`.
- **`fully-featured-pdf`** — no sidecar, and the file embeds third-party media (an
  MP3, a QuickTime movie, a U3D model) the README does not licence. Left on value first: one
  document, already complete, whose distinguishing half is Clause 13, which `CLAUDE.md` excludes.

**What it costs a fresh clone** is `du -sh doc/corpora/format-corpus` beside `doc/pdf.js` and
`doc/arlington-pdf-model`. **No `doc/todo/02` §2 gate requires it** — the tests that read it
(`crates/pdf-model/tests/collections.rs` among them) skip, saying so, where it is not checked out,
and `tools/safedocs survey --dir` reads it on purpose.

**And what `tools/safedocs` fetches is under no grant at all**, which is why `.gitignore`'s entry
for `corpus-cache/` carries a licence sentence: the `CC-MAIN-2021-31` corpus is eight million
PDFs crawled from the public web, and Common Crawl's own terms of use govern the *collection*
rather than the copyright in each document. Cache them, measure them, name them in a bug report;
do not commit one. ADR 0258's promotion budget exists for the one case where that rule and a
regression test collide.

**The same sentence binds SafeDocs' *Issue Tracker* corpus, and one more binds it harder.** That
set — under `corpus-cache/tika-issue-tracker/`, `doc/todo/03` §29 — is bug attachments gathered from 35
issue trackers of 32 PDF technologies. Every file in it belongs to whoever filed the report and
nobody granted anything; Apache hosted it for the tool developers and **stopped**, on takedown
requests, closing the question as `LEGAL-696` in April 2025. So the rule here is the crawl's rule
with its reason made sharper: cache them, measure them, name one in a commit message with its
digest, **commit none of them** — and the one route left to the bytes is the Internet Archive's
copy, each tarball checked against the SHA-512 Apache published beside it, which is what keeps a
third party's copy evidence rather than a guess.

**A dependency decision that came out *no* twice.** `tools/safedocs` reads HTTPS byte ranges and ZIP central
directories and takes **no package at all**: the transport is `curl` as a subprocess, so there is
no HTTP client, no TLS stack and no certificate store in this graph (the precedent is
`tools/pdfref` driving `pdftoppm`, `mutool` and `gs`), and the ZIP reading is `tools/safedocs/src/zip.rs`
because every ZIP crate reads `Read + Seek`, which over HTTP means writing the range-issuing
adapter that is most of the work anyway. `flate2` supplies the raw-deflate decoder *and* the CRC-32
the verification needs, and `sha2` is here for §7.6.

**Provenance is a principle-4 question**, and the tree has one precedent — `pdf-spec`'s Arlington
tables, built by `build.rs` from a pinned submodule. Vendored data arrives the same way: a
checked-in tool, a pinned upstream revision recorded beside the bytes, the licence file verbatim
next to what it covers.

**Every specification text below is held the same way unless its section says otherwise**: a PDF
under `doc/` (or `/home/AI/specs/`, ADR 1349) that `.gitignore`'s `/doc/*.pdf` excludes, its text
put by `tools/spec-md.py` into the ignored `doc/md/`, under ADR 0187's position — **free to obtain is
not free to redistribute**, so it is cited by section and paraphrased, in a code comment as much as
in a document, and never quoted.

## ISO 32000-1:2008, PDF/A-2's base document

The owner downloaded it on 2026-09-07 to `doc/PDF32000_2008.pdf`. Adobe publishes it without
charge — it is the edition ISO approved as ISO 32000-1:2008 — and it is held as every text here is.

**Why it matters here.** ISO 19005-2 section 5.1 makes a conforming PDF/A-2 file one that adheres to all
of ISO 32000-1 as modified by that part, and every "ISO 32000-1:2008, 9.x" citation part 2 makes
points into it, so a PDF/A-2 verdict leaning on a difference between the editions is written
against this one (`crates/pdf-archive`'s `Part::Two` says so; `doc/questions/Q49` asked for it). Two
things it holds are named where they act: Annex A's operator summary, and §9.7.5.2's predefined CMap
table, which ISO 32000-2 dropped and which PDF Association issue #77 is parked on.

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
`doc/pdfa/`.

## ISO/IEC 15444-1:2000, JPEG 2000's core coding system

The owner obtained it on 2026-09-07 as `doc/ISO_IEC 15444.pdf` — *Information technology — JPEG
2000 image coding system — Part 1: Core coding system*, the 2000 first edition.

ISO 19005-2 section 6.2.8.3's JPEG 2000 rules turn on fields no reader in this tree could see: Annex
I.5.3.3's `colr` box carries `METH`, `PREC`, `APPROX` and `EnumCS`, and Annex A.5.1's `SIZ`
marker segment carries the component count and each component's bit depth.

**What it does not carry.** §6.2.8.3 also asks that only the JPX baseline set of features be
used, and that is defined in ISO/IEC 15444-**2** M.9.2. Part 2 was not bought (`doc/questions/A51`);
its identical joint text is held instead, ITU-T T.801, entered below.

**Two files beside it that look like later editions and are not.**
`doc/ISO-IEC-15444-1-2016.pdf` and `doc/ISO-IEC-15444-1-2019.pdf` are the third and fourth
editions' **iTeh STANDARD PREVIEW** documents: title page, copyright notice, foreword and table of
contents, and nothing past them: their `doc/md/` text does not contain `colr` or `EnumCS`
(`grep -c`). They are recorded here so that a round reading the filenames does not spend an hour
discovering it.

**The edition trap is therefore permanent**, and it is recorded where it acts rather than only
here. ISO 19005 permits a `METH` of 3 where the 2000 edition defines only 1 and 2, names
enumerated colour spaces 12 and 19 where its Table I-10 defines only 16 and 17, and gives `APPROX`
a meaning where I.5.3.3 says the field shall be zero and readers shall ignore it. In all three the
rule implemented is ISO 19005's, and each constant in `crates/pdf-archive/src/table/graphics.rs`
says so above itself. All three are part 2's, and T.801 below defines them: Table M.22 the methods,
Table M.25 the enumerated spaces, Table M.23 the approximation.

## ITU-T T.801, the identical text of ISO/IEC 15444-2 — JPEG 2000's extensions

ISO 19005-2 section 6.2.8.3 and ISO 19005-4 section 6.2.7.3 send the JPX baseline to ISO/IEC
15444-2 M.9.2, and ISO 32000-2 §7.4.9 asks a processor to support its enumerated colour spaces. ITU
publishes the same text as Recommendation T.801 free of charge. It was fetched at the owner's
request on 2026-09-22 and is kept as `doc/T-REC-T.801-200208.pdf` — inside the oracle's and the
accessibility census's populations, so trap 43 is paid — and prepared with `python3 tools/spec-md.py
doc/T-REC-T.801-200208.pdf --out doc/md/T.801.md`, in the page's content order: the file has no
structure tree. ADR 1383 is
the reading and `doc/questions/A169` the owner's decision to read it.

| text | fetched from | SHA-256 of the PDF |
|---|---|---|
| **ITU-T T.801 (08/2002)**, *Information technology – JPEG 2000 image coding system: Extensions* | ITU-T's free-of-charge catalogue, `https://www.itu.int/rec/T-REC-T.801` (the 08/2002 edition) | `644bc4c41a6fb499482e560c0c9ad8da9f82621ab1971ce922aa5dc0bede21db` |

**Which edition, verified rather than assumed.** The title pages state ITU-T T.801 (08/2002),
approved on 29 August 2002, copyright ITU 2003; the source note says an identical text is also
published as ISO/IEC 15444-2, and every page's running head names ISO/IEC 15444-2:2003 (E). The
title pages do not themselves say that amendments or corrigenda are integrated. ITU's catalogue
page for T.801 does: it lists this edition with ISO/IEC 15444-2 Amendment 1 and Technical Corrigenda
1 and 2 integrated, and lists after it Corrigendum 3 (01/2005), Amendment 2 (05/2005), Corrigendum
4 (05/2006), Amendments 4 (06/2012) and 3 (03/2013), and new editions T.801 (06/2021) and
(08/2023). ISO 19005-2's normative reference is ISO/IEC 15444-2:2004. **The revisit condition is
the owner's (A169): the 2004 ISO edition, read only if the editions are ever found to differ on
M.9.2's baseline set**; the later corrigenda, amendments and editions above are where such a
difference would be found, and none of them has been read.

**Free to obtain, not free to quote.** The notice on its fourth page reserves all rights and
permits no reproduction by any means without ITU's prior written permission — the position T.4
and T.6 take below. So T.801 is cited by clause, table and figure and paraphrased, in source and in
the ledger alike, and nothing from it appears between quotation marks or after a `>`. The numbers
the code carries from it — box types, marker codes, `EnumCS` values, `Rsiz` bits, the `IL` codes of
Table M.29 — are the format itself rather than prose about it.

## ISO 15076-1:2010, the ICC profile format — a preview, and what it cannot answer

The owner downloaded it on 2026-09-10 as `doc/ISO-15076-1-2010.pdf` — *Image technology colour
management — Architecture, profile format and data structure — Part 1: Based on ICC.1:2010*.

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

**What the preview cannot give** is what a profile stating version 4.3.0.0 has to satisfy —
every requirement of clause 7 onward — which is why a profile naming that edition is judged by
nothing here; the `Profile ID` and the version field are answered by ICC.1:2022 and ICC.2:2023
below.

## ICC.1:2022, the ICC profile format — the whole text

The owner downloaded it on 2026-09-10 as `doc/ICC.1-2022-05.pdf` — *Image technology colour
management — Architecture, profile format, and data structure*, the International Color
Consortium's own edition, profile version 4.4.0.0. **It is complete**, clause 1 through Annex G —
the profile header field by field, the tag table, every tag and every tag type, and the
required-tag lists of clause 8.

**Its redistribution terms are not established.** The ICC's specifications are free to obtain and
that is not the same as free to redistribute, which is ADR 0187's distinction and the reason
`doc/md/` is ignored. Nothing was found stating what may be republished, so it is treated exactly
as `doc/pdfa/`'s two files are — **licensed to a single reader** — and **nothing in this tree
quotes it**: cite and paraphrase, in the code as much as in the documents. Any code comment
reproducing a sentence of it would be the thing this position exists to prevent.

**What it answers**:

- **Section 7.2.18, the `Profile ID` field.** Bytes 84 to 99, holding RFC 1321's MD5 over the
  whole profile — as long as the header's size field says — with the profile flags, the rendering
  intent and the field itself zeroed for the calculation, and a zero field meaning no identifier
  has been calculated. That is `pdf_model::icc::computed_id`, which decides the corpus witness
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
5.0.0.0, complete, and under the same unestablished terms as ICC.1:2022 above. **What it
answers**:

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
  section 7.2.20's method gives either. Both are reported, under both parts.
- **Section 7.2.20 is section 7.2.18 again**, word for word on the calculation, which is the second
  held witness for the method ISO 19005-4 cites.

**What it does not answer.** Everything about a profile stating any other version — and, for a
version 5 profile, everything this project has not read of its clauses 9 and 10.

## ICC.1:1998-09 and ICC.1:2001-12 — two of the four texts ISO 19005-2 names

The owner downloaded both on 2026-09-10, as `doc/ICC-1_1998-09.pdf` (*File Format for Color
Profiles*, profile version 2.2.0) and `doc/icc_1_2001-12.pdf` (*File Format for Color Profiles
(Version 4.0.0)*), each with its `doc/md/` text. Both are complete texts, under the same unestablished
terms as the two editions above.

**They are the first two of ISO 19005-2 section 6.2.4.2's four**, which is what makes them
different from ICC.1:2022 and ICC.2:2023. That clause's list stands at ICC.1:1998-09 **held**,
ICC.1:2001-12 **held**, ICC.1:2003-09 **not held**, ISO 15076-1 **preview only**.

**The second file is prepared in a mixture of both page orders**, which its own header records. It
is readable clause by clause all the same; a round quoting a line number from it should check the
surrounding page markers rather than assume the file runs forwards.

**What they answer**:

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
- **That the shipped `data/icc/sRGB2014.icc` is admissible under ISO 19005-2**: the profile's header states 2.0.0, and ICC.1:1998-09 states its own
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

## Black point compensation: the four texts, and which one states the algorithm

ISO 32000-2 §8.6.5.9 says that where `/UseBlackPtComp` is `ON` the conversion shall follow ISO
18619. None of the ICC texts above states that algorithm — ADR 1208 is the checking that closed
the family — and the ICC publishes the committee draft that does, free. This section is the
provenance of all four.

**The freely redistributable one: PDF 2.0 Application Note 001.**
`doc/md/PDF20_AN001-BPC.md` is *Black Point Compensation*, PDF Association, 2018-09, and its cover
page states Creative Commons Attribution 4.0 International — **the one text in this family that is
freely redistributable, with attribution**. It is also a corpus document several crates open by
name. What it defines is one paragraph of prose: compensation aligns the darkest colour the source
data's space can express with the darkest the output device's profile can produce, by analogy with
what absolute colorimetry does at the white end. For the algorithm it defers to ISO 18619 in as
many words, and its reference list names exactly two documents, ISO 32000-2 and ISO 18619. So the
note settles what the flag *means* and states nothing computable.

**Free to obtain from the ICC, and held here.** Three documents, fetched on 2026-09-22; the byte
counts below are what came back and what a re-fetch should match:

| document | file, and where it came from | terms |
|---|---|---|
| ISO/CD 18619, *Image technology colour management — Black point compensation* (ISO/TC 130/WG 7 N063, 2013-05-02) | `doc/ISO-CD-18619-2013.pdf`, `doc/md/ISO-CD-18619-2013.md`; `https://www.color.org/BlackPointCompensation.pdf`, 340 916 bytes | ISO committee-draft notice: reproduction permitted for participants in the ISO standards development process and **for no other purpose without ISO's written permission** — the same single-reader position as ICC.1:2022 above |
| ICC White Paper 40, *Black-point compensation: theory and application* (2010-07-27) | `doc/ICC-White-Paper-40-BPC.pdf`, `doc/md/ICC-White-Paper-40-BPC.md`; `https://archive.color.org/files/WP40-Black_Point_Compensation_2010-07-27.pdf`, linked from `https://www.color.org/whitepapers/`, 2 338 582 bytes | **no copyright or permission statement in the document at all**; terms unestablished, treated as single-reader |
| *Adobe Systems' Implementation of Black Point Compensation* (2006) | `doc/AdobeBPC-2006.pdf`, `doc/md/AdobeBPC-2006.md`; `https://www.color.org/AdobeBPC.pdf`, 196 694 bytes | Adobe copyright, no grant of any kind — and one sentence that matters: Adobe states it holds **no patents** on the algorithm or on its use in colour transforms |

**What the draft states, and where it lives in this tree.** Its clause 4 is the whole
procedure — a vertex set per colour space, a branch for an output-capable CMYK source, a black
point clamped in lightness, an estimation for a LUT-based *destination*, and one scale and offset
in PCSXYZ — and `crates/pdf-colour/src/icc.rs` carries it out for a source profile against the
display. ADR 1253 is the reading and the measurement; `Profile::source_black_point`'s doc comment
is the section-by-section paraphrase.

**Three cautions, and each changes what a round may write.**

- The free ISO document is the **2013 committee draft**, not ISO 18619:2015. Its cover states it is
  the final approved ICC version about to be balloted, which is the strongest provenance available
  without buying the standard — so a citation says *ISO/CD 18619 (2013)* and never claims the
  published text.
- **Nothing here may be quoted**, the application note excepted. The ISO draft's notice forbids
  reproduction outright and the white paper states no terms, which under ADR 0187 is the same
  answer. Cite by section and paraphrase, in a code comment as much as in a document.
- **The white paper is the cheapest citation for the arithmetic.** Its section 5 states the
  mapping — a scale factor from the two black points' luminances and an offset towards the D50
  white — with no ISO notice attached to it, where the draft states the same relation under one.
  An equation is method rather than expression in any case, but the round that writes one should
  know which document it is standing on.

## The two ICC profiles this tree ships

Not texts but data, and one of two sets of vendored bytes a *converted document* carries away with
it: a PDF/A output intent this converter adds embeds one of them. The other set is below.

| file | source | terms, from its own `cprt` tag |
|---|---|---|
| `data/icc/sRGB2014.icc` | the ICC's RGB registry, 2026-09-10 | the ICC's own grant: copied, distributed, embedded, made, used and sold without restriction; an altered copy must drop the identification and copyright |
| `data/icc/GRACoL2006_Coated1v2.icc` | the ICC's profile registry, 2026-09-21 | IDEAlliance's, with X-Rite's permission: used, embedded, exchanged and shared without restriction; **not altered**, and not sold without IDEAlliance's written permission |

Both are shipped byte for byte, which is what each grant turns on, and
`data/icc/PROVENANCE.md` carries the hashes, the header fields read out of each file, the edition
argument per class, and — for the CMYK one — the four alternatives declined and why. The licence
sentence travels to the person who embeds it: the conversion's report prints the `cprt` tag of
whichever profile the output intent names (`doc/pdf-a-conversion-limits.md` section 10.1).

**`cargo deny` cannot see either of them**, for the same reason `viewer-ui/tests/notices.rs` exists
for the fonts: it reads Cargo metadata and these are vendored bytes. What watches them is
`data/icc/PROVENANCE.md`'s hashes and the tests in `crates/pdf-colour/src/icc.rs` and
`crates/pdf-archive/src/table/graphics.rs` that read each file's own tags.

## The Adobe CMaps, when a conversion embeds one

A PDF/A conversion answering `fonts/cmap-embedded-or-predefined` with `source = "shipped-cmaps"`
writes one of `data/cmaps/`'s programs into the output as a CMap stream (ADR 1286). The terms are
the BSD-3-Clause row above, and the obligation that matters for a redistributed copy — reproduce
the notice and disclaimer — is met by the bytes themselves: every program carries Adobe's notice in
its own `%%Copyright` header comments, and the stream holds the program unchanged, so the notice
leaves with it. Nothing is re-wrapped, stripped or re-serialised, which is the condition this rests
on; the conversion's report names every CMap embedded.

## The XMP Specification, read for a table of facts

ISO 19005-2 section 6.6.2.3.1 requires every XMP property to come from a predefined schema, and deciding
that needs to know what those schemas *contain* — the corpus's witnesses for the clause are
almost all a predefined property carrying the wrong value type, not an unknown property. The
standard names its source in its own bibliography, entry [20]: *XMP: Extensible Metadata Platform,
September 2005, Adobe Systems*. The AIIM address it gives is dead; the document was read from the
Internet Archive, and the current edition (XMP Specification Part 2, *Additional Properties*, 2017)
beside it.

**What was taken is a table of facts, not text.** `crates/pdf-archive/src/table/metadata.rs`'s
`PREDEFINED` carries the property names of the predefined schemas, each reduced to the two things a reader
can check — a shape (simple, language alternative, alternative, seq, bag, structure) and a lexical
form (Boolean, Integer, Real, Rational, Date, GPS coordinate, or none). That is the same kind of
taking as the standard-14 metrics in `crates/pdf-font/src/standard_metrics.rs`: a list of values a
specification states, not a passage of its prose. **Nothing is quoted**.

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
properties*, licensed to a single reader like `doc/pdfa/`'s two files.

**It is an iTeh STANDARD PREVIEW too, and this one carries real clause text.** Clause 1 through
section 7.2 — the scope, the normative references, the terms, the notations, clause 5's
conformance, the whole of clause 6's data model, and the general and equivalent-forms preamble of
clause 7's serialisation. It stops there. Sections 7.4 to 7.8, which are the canonical RDF
serialisation, and section 7.9, which says which equivalent RDF forms are allowed and prohibited,
are **not** in it; neither is clause 8's core properties, whose tables this tree reads from Adobe's
own edition instead (the section above).

**What it settles**:

- ISO 19005-2 section 6.6.2.1's and ISO 19005-4 section 6.7.2.1's serialisation is five rows — two
  checks over the data model and the single `rdf:RDF` element, two naming held sentences
  `pdf_model::xmp` cannot see, and one keeping what the preview does not reach. `crates/pdf-archive/src/table/metadata.rs`'s module comment has the split and the reason
  the data-model row binds part 2 alone.
- **The encoding of an XMP packet is nobody's rule.** Section 7.1 names UTF-8, UTF-16 and UTF-32,
  puts the choice between them beyond its own scope, and leaves it to whichever standard embeds
  the packet — and neither ISO 19005 part nor ISO 32000-2 §14.3.2 states one. That decides the
  corpus witness `6-7-2-1-t01-fail-e`, whose expectation is that a packet not encoded as UTF-8
  fails; `crates/pdf-archive/tests/corpus.rs` carries the ruling and names the three documents
  read.
- **A comparison of `xml:lang` values is case-insensitive** (section 6.4, by way of IETF RFC
  3066), so a packet writing `X-Default` has the title `pdf_model::xmp` finds.
- The three permitted encodings are section 7.1's, which `pdf_model::xmp` cites.

**What it cannot answer.** Which RDF spellings a packet may use and which it may not — that is
sections 7.4 to 7.9 — so `metadata/xmp-packets-meet-the-xmp-serialisation` stays `Unchecked` with
that as its reason. A round wanting it needs the full text; the preview will not grow.

## ISO 19444-1:2019, XFDF — a preview that reaches section 5.7.1

Fetched on 2026-09-15 as `doc/ISO-19444-1-2019-preview.pdf` from
`https://www.sis.se/api/document/preview/80014076/` — *Document management — XML Forms Data Format
— Part 1: Use of ISO 32000-2 (XFDF 3.0)*, second edition. **A licensed ISO text, held as the
previews above are** — `pdf_model::xfdf`, `pdf_model::submission`'s writer, their tests, ADR 1108 and the
ledger rows for §12.7.6, §12.7.6.2, §12.7.6.4, §12.7.8 and §12.7.8.3.4 all cite sections and quote
nothing. The one thing reproduced from it is the *format's* bytes rather than the standard's prose:
section 5.5.2 settles the two lines an XFDF document begins with, and those are what a writer of
the format writes.

**Adobe's free route is gone.** ISO 32000-2 cites XFDF as ISO 19444-1, which is paid; Adobe published
the same document free as *XML Forms Data Format Specification, version 3.0* (2009), and no longer
does: `https://www.adobe.com/go/xfdf_spec` answers `302` to `/404.html`,
`opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/` has no `XFDF_Spec_3.0.pdf`, and the PDF
Association's normative-references page links XFDF to `iso.org`'s paywall while hosting XFA 3.3
itself (the Internet Archive's copy is the next section's). A re-host on a document-sharing site carries the text as scraped page images and is not a copy
anybody should read as a specification. ISO's own previews are, and there are three of them:
`cdn.standards.iteh.ai`'s 2019 sample reaches body page 5, its 2016 sample reaches page 7, and
sis.se's reaches **page 9** — which is the one kept.

**What those nine pages carry**, and it is the whole of the field half of the format: the complete
table of contents, section 5.4's PDF/FDF/XFDF relationship with the XFDF-to-FDF key mapping,
section 5.5.2's encoding and namespace `shall`s, section 5.6's flat and hierarchical form examples,
and section 5.7.1's one sentence separating annotations from forms. `pdf_model::xfdf` and
`pdf_model::submission`'s XFDF writer are written against it, which ADR 1108 argues.

**What it cannot answer.** Sections 6.2 to 6.7 — the element reference, the annotation elements,
the annotation attributes and the PDF-to-XFDF mapping tables — are the other ninety pages and are
not in any preview; the preview will not grow. That material is read from the document the
standard was made from, the next section.

## XML Forms Data Format Specification 3.0 (Adobe, 2009) — the text ISO 19444-1 was made from

Held as `doc/XFDF_Spec_3.0.pdf`: Adobe's *XML Forms Data Format Specification*, version 3.0,
August 2009, describing XFDF for Adobe Extension Level 3 to ISO 32000-1 and Acrobat 9.
The owner fetched it on 2026-09-22 from Adobe's own free distribution of it by way of the Internet
Archive, and `doc/questions/A97` records that and the owner's direction to hold it rather than buy
ISO 19444-1. Its `doc/md/XFDF_Spec_3.0.md` is in the structure tree's order.

**It is the document ISO 19444-1 was made from**, which is why it is held: the preview above
carries ISO 19444-1:2019's field half, sections 5.4 to 5.7.1, and this text carries what the preview
does not reach — chapter 2's whole element reference, the `annots` element and every annotation
element under it, the annotation attributes grouped by the PDF dictionary each belongs to, and the
two mapping tables. The two are not interchangeable. ISO 19444-1:2019 is a second edition written
against ISO 32000-2, and this is Adobe's edition written against ISO 32000-1; a later round that
finds the two differ materially in ISO 19444-1's sections 6.2 to 6.7 has the revisit condition
`A97` names, and the purchase is catalogue number 74272.

Its headings below chapter level are not numbered, so **it is cited by chapter, heading and page,
paraphrased and never quoted** — *XFDF 3.0, chapter 2, Annotation attributes, page 70* — never with
a `§`, never in a `>` blockquote, and never inside a comment in its own words. Nothing of it is committed: the fixtures
under `crates/pdf-model/tests/xfdf/` are files written for the tests, in the format's grammar with
values of their own. Where it and ISO 32000-2 disagree about an entry, ISO 32000-2 wins, and ADR
1297 lists each place.

**What it settled.** `pdf_model::xfdf` reads `<annots>` into the same annotations an FDF file's
`/Annots` is read into, so both formats are placed by one import (ADR 1297). Its extraction was
checked for the element reference and the annotation material before anything was built on it:
chapter 2's *Annotation Elements*, *Annotation Subelements* and *Annotation attributes* and both
mapping tables are present and in order, pages 37 to 99. Appendix A, the `ex_data` element for
comments on 3D and rich media annotations, is read and refused: its subject is clause 13's.

## XML Forms Architecture (XFA) Specification 3.3 — the rich text §12.7.4.3 hands its formatting to

ISO 32000-2 names one text for every rich text string it defines — Table 228's `/RV` and `/DS`,
Table 249's `/RV`, Table 172's and Table 177's `/RC`, Table 177's `/DS` — and it is *Adobe XML
Architecture, XML Forms Architecture (XFA) Specification, version 3.3*. The copy held is the PDF
the PDF Association hosts among ISO 32000-2's normative references, taken from the Internet
Archive's capture of 2026-08-19, and kept outside the tree at `/home/AI/specs/XFA-3_3.pdf` on the
agent's machine; `python3 tools/spec-md.py /home/AI/specs/XFA-3_3.pdf --out doc/md/XFA-3_3.md` put
its text under the ignored `doc/md/`, in the page's content order (it carries no structure tree):

| text | SHA-256 of the PDF |
|---|---|
| **XFA Specification, version 3.3**, dated 9 January 2012 | `a3344e7ef0b0da445bcce4323e689e646b31b64ecf1c318a8fc98f6b5edca01e` |

**What its notice permits, paraphrased.** The preface's *Intellectual Property* section (page ix)
keeps Adobe's copyright in the grammar and the processing rules and grants a permission to use the
architecture for four purposes, one of them writing software that accepts input in the
architecture and displays, prints or otherwise interprets its contents; the permission extends to
copying the grammar and the example code as far as those purposes need, on condition that the
copyright notice goes with what is copied. It grants no right to reproduce the specification's
prose. So it is held the way XFDF 3.0 is (ADR 0187): **cited by chapter, section heading and page,
paraphrased, never quoted** — *XFA 3.3 section 27, Supported Character Formatting, page 1198* —
never with a `§`, never in a `>` blockquote, and nothing of it committed. The fixtures under
`crates/pdf-model/tests/rich_text.rs` and `crates/pdf-model/src/rich_text/` are written for the
tests, in XHTML's and CSS's own grammar with words and values of their own, each naming the example
of chapter 27 whose rule it holds; none copies an example's text, so the notice's condition is
never reached. The element and property names it lists are XHTML's and CSS2's, which the chapter
names as its normative references for their values.

**What it is read for.** Chapter 27, the Rich Text Reference (pages 1187 to 1223), and the
measurement grammar of chapter 2's *Basic Composition* (page 36) that the chapter's relative
measurements point at. `pdf_model::rich_text` is written against it (ADRs 1634, 1635). Nothing
else in the specification is read: the template grammar, the data binding, the scripting and the
layout of content areas are Annex K's schema-driven page generation, which `CLAUDE.md`'s XFA
exclusion declines on §K.1's permission, and that exclusion does not reach a rich text string an
AcroForm entry or an annotation carries (ADR 1197). Part 4, *Adobe Implementation*, is one
vendor's departures and is evidence at most.

## PDF Association TechNote 0010, and a licence that could not be confirmed

The owner obtained it on 2026-09-09 as `doc/TechNote0010.pdf` — *TechNote 0010: Clarifications of
ISO 19005, parts 1-3 for developers of PDF/A creators and validators*, PDF Association, 2017,
items each carrying a resolution of the ISO working group responsible for ISO 19005.
`doc/adr/0931` says what this tree does with a clarification that is neither the standard nor an
erratum, `doc/adr/0933` which parts a clarification reaches, and
`crates/pdf-archive/src/clarification.rs` is where every item taken from it lives, beside the ones
recorded as confirming a reading, as owed, or as bearing on no row at all.

| data | source examined | terms |
|---|---|---|
| PDF Association TechNote 0010 (2017) | the document itself, page 1 and its XMP packet | **stated copyright, no stated grant** — see below |

**What the document says about itself is one line.** Page 1 carries a copyright notice naming the
year and nothing else: no licence, no permission, no terms of use. Its XMP packet carries no
`dc:rights` and no rights-management properties at all — which is worth recording precisely
because the file is a conforming PDF/A-2a document whose producer filled in everything else.

**What the publisher says could not be reached.** `pdfa.org` returns HTTP 403 to this machine, to
both a browser user agent and a plain fetch, so the resource page's own terms are unread. `doc/rfc/0006` records the document as CC-BY-4.0, and a
web search agrees, but **neither is a licence text this project has seen** — a second-hand report
of a licence is the same kind of evidence as a second-hand report of a clause, and principle 5's
discipline does not change subject just because the subject is copyright.

**So the terms are treated as unclear, and the consequence is a rule rather than a worry: nothing
in this tree quotes it.** Every one of its resolutions is cited by item number and paraphrased in
this project's own words — `TechNote 0010 section A021`, never a sentence of it between quotation
marks — which costs nothing, because what a validator needs from the note is the rule and not the wording. If the
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
the standard's forms rather than the XMP Specification's six profiles of them.
`crates/pdf-archive/src/iso_8601.rs` is what was read from it, cited to its clause numbers.

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
this crate's own sentences under the draft's clause numbers, and the draft's own example values (`19850412`,
`1985-W15-5`) appear in tests as data rather than as prose.

## The three ETSI European Norms, and the strictest notice on this disk

ETSI publishes its European Norms at no charge from `https://www.etsi.org/deliver/etsi_en/`, and
three of them are on this disk, held as every text here is:

| text | what cites it |
|---|---|
| **EN 319 122-1 V1.3.1 (2023-06)**, CAdES digital signatures, part 1 | §12.8.3.4.3's rules, which the clause states only by reference to it |
| **EN 319 122-2 V1.1.1 (2016-04)**, CAdES baseline signatures | §12.8.3.4.4's profiles, whose Table 1 cardinalities are that document's |
| **EN 319 102-1 V1.4.1**, signature creation and validation procedures | `pdf_signature::verdict` and `pdf_signature::revocation` — the vocabulary in which a verdict about a signature is stated at all |

**Their notice is the strictest here and it changes how the tree cites them.** It permits no
reproduction in any form without ETSI's written permission. So clause numbers are cited and the
rules are paraphrased — in source, in the ledger and in the documents alike — and **nothing from any
of the three appears between quotation marks or after a `>`**. ISO 32000-2's own sentences are
quoted as always, because the conformance checker verifies those against `doc/md/`, and the two are
kept visibly apart. ADR 1085 is the argument and what holding them bought.

## veraPDF, and the difference between running a program and reading it

The owner put a checkout of veraPDF in `doc/veraPDF-library` on 2026-09-07 and asked whether its
licence lets this project look at the code. It is dual-licensed **GPLv3+ or MPLv2+** — both
licence texts are in the checkout, as `LICENSE.GPL` and `LICENSE.MPL`, and `LICENSE-HEADERS.md`
says the dual licensing is deliberate and applies to all veraPDF software. `doc/.gitignore`
excludes the directory, as the root `.gitignore` excludes `/doc/rasterrocket`, and for the same
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
   from veraPDF's profiles is somebody else's reading of a text — and `doc/pdfa/` holds ISO
   19005-2 and -4 themselves, a better source than any implementation of them. So the rule for this tree is the one principle 5
   already implies:

   > **Run veraPDF; do not read it.** Where it disagrees with `pdf-archive`, read the clause it
   > cites in `doc/pdfa/`, not the code that cites it.

   That rule is cheap to keep and it removes the derivation question entirely, which is the
   pleasant part: the licence-safe path and the principled path are the same path.

**Its test files are a separate question, and this checkout has none**: `find doc/veraPDF-library
-iname '*.pdf'` returns nothing, and `core/src/test/resources` holds XML fragments (XMP packets and
policy reports), text files and schemas — the inputs to unit tests of the XMP and policy readers,
not documents. The validation profiles are not here either; the checkout carries
`validationProfile.xsd`, the schema they are written against, and nothing written in it. veraPDF
splits those across repositories, so a corpus and a profile set are each a separate fetch with a
separate licence to read.

**The corpus is a *separate repository* from the library and carries a *different* licence**: `doc/veraPDF-corpus/README.md`
states **Creative Commons Attribution 4.0 International (CC BY 4.0)** — permissive, no copyleft,
redistribution allowed with attribution. So none of the GPL/MPL question above touches it, and it
is a **submodule** under `doc/veraPDF-corpus` rather than an ignored directory: the four corpora
under `doc/corpora/` are tracked the same way, and a submodule carries no bytes into this history.

What it holds (`find doc/veraPDF-corpus -name '*.pdf' | wc -l` counts them) is documents in directories named by clause
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
what `crates/pdf-archive/tests/verdict.rs` does. Our own fixtures are
the better *evidence*; somebody else's corpus is the better *coverage*. The two answer the two
questions `CLAUDE.md` keeps apart, and neither substitutes for the other.

**Nothing of the library is taken or read** — examined, and not taken, as the `cidToUnicode` row above.

## ITU-T T.4 and T.6, the fax codings §7.4.6 hands its filter to

ISO 32000-2 §7.4.6 says the `CCITTFaxDecode` coding "is not described in detail in this document"
and names two ITU-T Recommendations for it. ITU publishes both free of charge, and both were fetched
on 2026-09-28 into a round's scratch directory, never under `doc/` (a PDF there joins the oracle's
population, trap 43),
and prepared with `python3 tools/spec-md.py <pdf> --out doc/md/T.4.md` and `--out doc/md/T.6.md`
into the ignored `doc/md/`. The PDFs are kept outside the tree, at `/home/AI/specs/t4.pdf` and
`/home/AI/specs/t6.pdf` on the agent's machine, and the hashes below are theirs — which is how a copy
fetched again is known to be the one read:

| text | fetched from | SHA-256 of the PDF |
|---|---|---|
| **ITU-T T.4 (07/2003)**, *Standardization of Group 3 facsimile terminals for document transmission* — the edition in force | `https://www.itu.int/rec/dologin_pub.asp?lang=e&id=T-REC-T.4-200307-I!!PDF-E&type=items` | `84c66fab164990bf48cb127e6f43598197d6260cd4ae533c190cbb14edac74f8` |
| **ITU-T T.6 (11/1988)**, *Facsimile coding schemes and coding control functions for Group 4 facsimile apparatus* — the Blue Book extract, the only edition | `https://www.itu.int/rec/dologin_pub.asp?lang=e&id=T-REC-T.6-198811-I!!PDF-E&type=items` | `8ccbe2d203e03ee2ac37478d50d2d946bab49dfb45376eb0f773009d183a7767` |

**Free to obtain, not free to quote.** Both carry ITU's copyright notice — T.4's dated 2004, T.6's
1988 and 1993 — reserving all rights and permitting no reproduction without ITU's prior written
permission. So they are held the way the ETSI norms above are: sections and figures are cited by
number and paraphrased, in source and in the ledger alike, and nothing from either appears between
quotation marks or after a `>`. The one thing `crates/pdf-ccitt` does carry from them is the code
tables — T.4 Tables 2, 3a, 3b and 4 as strings of binary digits — which are the coding itself
rather than prose about it: every fax decoder carries them, `hayro-ccitt` among them, and a decoder
cannot be written without them. Every run-length code was checked against the extracted text
and, as evidence, against `hayro-ccitt`'s tables, and agrees with both; `codes.rs`'s tests hold them
to being prefix-free. T.4 Figures 10 and 11's coding examples are encoded as fixtures in
`crates/pdf-ccitt/tests/coding.rs` (ADR 1349).

## ITU-T T.88, the identical text of ISO/IEC 14492 — JBIG2, which §7.4.7 hands its filter to

ISO 32000-2 §7.4.7 defines `JBIG2Decode` by reference to ISO/IEC 14492, and this tree has cited that
standard's clauses from `hayro-jbig2`'s comments and its own reading of section 7.2 without holding
the text. ITU publishes the identical text as Recommendation T.88 free of charge; the 08/2018
edition, the one in force, is offered only as a zip that also carries the Recommendation's
conformance data and sample software. It was fetched on 2026-10-01 into a round's scratch directory; the PDF inside it is kept outside the
tree at `/home/AI/specs/T.88.pdf` and prepared with `python3 tools/spec-md.py /home/AI/specs/T.88.pdf
--out doc/md/T.88.md` into the ignored `doc/md/`, in the structure tree's order:

| text | fetched from | SHA-256 |
|---|---|---|
| **ITU-T T.88 (08/2018)**, *Information technology – Lossy/lossless coding of bi-level images* — the zip | `https://www.itu.int/rec/dologin_pub.asp?lang=e&id=T-REC-T.88-201808-I!!SOFT-ZST-E&type=items` | `e2a3c458c4d28d5a835c0815fd9b23e197dbc1b83fe81d8e8d8b2ac413d8f22f` |
| the Recommendation's PDF inside it, `ITU-T_T_88__08_2018.pdf` | the zip above | `dff16153d2b959d78e0018bfc0a054bd4ba136b0a156021b03e4743bfa6af2d6` |

**Free to obtain, not free to quote.** The text carries ITU's 2019 notice reserving all rights and
permitting no reproduction without ITU's prior written permission — the position T.4, T.6 and T.801
take above. So T.88 is cited by section, table and figure and paraphrased, in source, in patches and
in the ledger alike, and nothing from it appears between quotation marks or after a `>`.

**The conformance data and sample software are read, not taken.** Annex K's files come with a
copyright notice of their own (ICT Link, `Software/Copyright Notice.txt` in the zip) licensing them
for inclusion in, evaluation for, and conformance testing of an implementation of the
Recommendation — narrower than the terms this repository is offered under, so nothing of either is
in it. The zip is kept at `/home/AI/specs/T.88-201808.zip` (the hash above) and unpacked under
`/home/AI/specs/T.88/`, the conformance data to
`Software/conf/JBIG2_ConformanceData-A20180829/` and the sample software to `Software/soft/`.
`crates/pdf-sandbox/tests/t88_conformance.rs` reads the ten `.jb2` streams and their reference
`.bmp` files from there, or from `$T88_CONFORMANCE_DATA`, and passes with a printed sentence where
neither holds them (ADR 1459). They stay outside the repository: the owner answered
`doc/questions/Q209` by its recommendation (`A209`), and committing them beside their notice under
a licence-file exclusion is the shape that answer leaves available. The sample software's source was read as evidence about what the streams' encoder
did — never as a definition of T.88 — and the test carries two bytes of Annex H.1's datastream,
the place where the conformance file departs from it.

## ITU-T T.81, the identical text of ISO/IEC 10918-1 — the JPEG process §7.4.8 hands `DCTDecode` to

ISO 32000-2 §7.4.8 defines `DCTDecode` by reference to ISO/IEC 10918-1, and this tree's band
decoders (`crates/pdf-model/src/image/dct/restart.rs`, `cut.rs`) cut a codestream on that
standard's entropy-coding rules. ITU publishes the identical text as Recommendation T.81 (09/92);
its catalogue offers the PDF only behind a login, so the copy the W3C hosts was fetched on
2026-10-01, kept outside the tree at `/home/AI/specs/T.81.pdf` and prepared with `python3
tools/spec-md.py /home/AI/specs/T.81.pdf --out doc/md/T.81.md` into the ignored `doc/md/`:

| text | fetched from | SHA-256 |
|---|---|---|
| **CCITT Rec. T.81 (09/92)**, *Information technology – Digital compression and coding of continuous-tone still images – Requirements and guidelines* | `https://www.w3.org/Graphics/JPEG/itu-t81.pdf` | `631031d4ba56b06abee3e312a0f235b9422da9c7267d1c8f7604418795768bf0` |

**Free to obtain, not free to quote.** The text reserves all rights and permits no reproduction
without the ITU's written permission, as T.4, T.6, T.88 and T.801 do above. So T.81 is cited by
section, table and figure and paraphrased, and nothing from it appears between quotation marks or
after a `>`.

## IETF RFCs 5639, 7027 and 8032, the two curves this tree computes itself

The owner's answer `doc/questions/A170` has the tree compute brainpoolP512r1 and Ed448 itself
(ADRs 1385 and 1386), so three RFCs supply **data** that is in source, not only prose read:
RFC 5639 section 3.7's six curve constants (`crates/pdf-signature/src/brainpool_p512.rs`), RFC 8032
section 5.2's Table 2 constants (`crates/pdf-signature/src/ed448.rs`) and its section 7.4 and 7.5
test vectors (`crates/pdf-signature/src/ed448/vectors.rs`), and RFC 7027 appendix A.3's
key-agreement values (`brainpool_p512.rs`'s tests). RFC 5639 and RFC 8032 are in `doc/md/rfc/`.
RFC 7027 is not: it was fetched on 2026-09-28 from `https://www.rfc-editor.org/rfc/rfc7027.txt`
(SHA-256 `28aaaff85a4d52b426194839b222e314fa6476ec75d7fa70427724710696a1f5`) into a round's scratch
directory, because `doc/md/` in a worktree is the owner's checkout.

**Terms, read rather than assumed.** Each carries the IETF Trust's notice (BCP 78 and the Trust
Legal Provisions). TLP 5.0 (fetched 2026-09-28 from the Trust's site as
`Corrected-TLP-5.0-legal-provsions.pdf`, SHA-256
`b5cbd993d1dae273d7b251db4efc397b4de13451b2156b94d5a901c83c0fb48a`) section 3.c.iii licenses anyone
to copy and distribute unmodified portions of an RFC provided each is attributed to the IETF and
names the RFC it came from, the legends being owed only past a fifth of a document's text; section
8 applies that section to the IRTF stream (RFC 8032) and the Independent Submission stream (RFC
5639) as to the IETF's own (RFC 7027), excluding only the Code Components licence of section 4. The
values are unmodified but for joining the lines they were printed on, each file names its RFC, and
none is near a fifth of its document. Prose from the three is paraphrased in source, never quoted,
because the quotation gate reads quotation marks as ISO 32000-2's.

## The Unicode Character Database, and `unicode-bidi`, for a right-to-left field value

§12.7.4.3's construction draws a field's value, and a value in Arabic needs the Unicode Standard's
order and joining (ADRs 1413 and 1414). Everything read was fetched on 2026-09-29, into a round's
scratch directory first: the UCD's `ArabicShaping.txt`, `extracted/DerivedJoiningType.txt`,
`UnicodeData.txt`, `BidiMirroring.txt` and `BidiCharacterTest.txt` (version 18.0.0, each from
`https://www.unicode.org/Public/UCD/latest/ucd/`), UAX #9 revision 52 from
`https://www.unicode.org/reports/tr9/`, the core specification's chapter 9 (the joining and
ligature rules, section 9.2) from `https://www.unicode.org/versions/latest/core-spec/chapter-9/`,
and the licence from `https://www.unicode.org/license.txt`.

**Terms, read rather than assumed.** The licence is UNICODE LICENSE V3 — the text `/NOTICE`
section 5 already carries verbatim for `CaseFolding.txt` — which grants copying, modification and
distribution of the data files on the one condition that the notice appear with every copy or in
its documentation, and `cargo deny` admits it as `Unicode-3.0`. The five files are committed
unchanged under `data/unicode/`, with their hashes and the fields used in
`data/unicode/PROVENANCE.md`; four are compiled into `static` tables by `crates/pdf-font/build.rs`
and the fifth is read by a test. The annex and chapter 9 are cited by rule number and paraphrased in
source, never quoted, because the quotation gate reads quotation marks as ISO 32000-2's.

The CIE's open-access datasets — the 1931 2° colour-matching functions and the relative spectral
distributions of the illuminants ITU-T T.4 Annex E codes (A, C, D65, D75 and the fluorescent set) —
are held unchanged as CSV under `data/cie/`, CC BY-SA 4.0, with their sources and hashes in
`data/cie/PROVENANCE.md`. Nothing reads them at run time: a JPX CIE Lab box's white point is compiled
into `crates/pdf-model/src/jpeg2000.rs`, and a test recomputes each value from these files, which is
what makes the compiled numbers checkable (ADR 1713). D65 is §8.6.5.4's own EXAMPLE and D50 is T.4's.

**`unicode-bidi` 0.3.18, taken rather than written** (`doc/stack.md` has the dependency). Its coverage was measured rather than read off its README: every
line of `BidiCharacterTest.txt` resolves as the file states, through `pdf_font::shaping`, whose own
rule L1 sits between the crate's resolution and its reordering. Its tables are Unicode 16.0
against the file's 18.0, and no line disagrees.

**Not taken.** `rustybuzz`, for `doc/stack.md`'s reason, and `harfbuzz_rs`, which is C. The joining
is the Unicode Standard's own rules over the tables above, reaching each form as a presentation-form
code point; executing a face's `GSUB` instead is priced in ADR 1414 and not built.

## IOGP Guidance Note 7-2 and ISO 19162, the texts §12.10 hands its projection to

§12.10.3 names two things outside ISO 32000-2 for a coordinate system: an EPSG code, whose
registry the IOGP administers, and a WKT string whose format ISO 19162 specifies. The owner's answer
`doc/questions/A171` has the projection built of this tree's own, and ADR 1586's census decided that
no registry is carried (every system stating `/EPSG` states `/WKT` beside it). Two free texts were
fetched on 2026-10-06 into a round's scratch directory, never under `doc/` (trap 43), prepared with
`python3 tools/spec-md.py <pdf> --out doc/md/IOGP-373-07-02.md` and `--out doc/md/OGC-18-010r11.md`
into the ignored `doc/md/`, and kept at `/home/AI/specs/` on the agent's machine:

| text | fetched from | SHA-256 of the PDF |
|---|---|---|
| **IOGP Publication 373-7-2, Geomatics Guidance Note 7 part 2**, *Coordinate Conversions and Transformations including Formulas*, revised September 2019 | `https://www.iogp.org/wp-content/uploads/2019/09/373-07-02.pdf` | `8951d9641e6a65a071e865d1e6bb0611d0d7248772fa3054496fc14041183465` |
| **OGC 18-010r11**, *Geographic information — Well-known text representation of coordinate reference systems*, version 2.1.11 (2023), the OGC's publication of ISO 19162:2019 | `https://docs.ogc.org/is/18-010r11/18-010r11.pdf` | `c97a5eb8f454b7309c990e1e0eef18f884b74f7ade8cf7f1a6f15f43668430e3` |
| **OGC 01-009**, *OpenGIS Coordinate Transformation Services Implementation Specification*, revision 1.00 (2001), the text that defines the older WKT form, `TOWGS84` included; fetched on 2026-10-15 for the datum transformation of `doc/questions/A436` (ADR 1958) and kept at `/home/AI/specs/01-009.pdf`; `python3 tools/spec-md.py <pdf> --out doc/md/OGC-01-009.md` prepares its text, not yet run (`doc/md/` holds no `OGC-01-009.md`) | `https://docs.ogc.org/sap/01-009/01-009.pdf` | `3afb6f4f032f262a7397652aeeae992d3f157556772b2fc758031b2423c1c0f8` |

**Cited and paraphrased, never quoted.** The Guidance Note states no terms of reproduction in its
text, which under ADR 0187 is the same as none granted. The OGC texts carry a licence permitting
copies with their notices intact, but the quotation gate reads a quotation mark as ISO 32000-2's, so
all three are cited by section — "Guidance Note 7-2 section 3.2.3.1", "ISO 19162 section 6.4" — and
paraphrased in source and in the ledger alike. What `crates/pdf-model/src/geospatial/` carries from
the Note is its formulas, which are the method rather than prose about it, and its worked examples'
numbers as test inputs (ADR 1587). Nothing from the EPSG Dataset is carried: the parameter and
method codes `system.rs` recognises are the identifiers the Note itself prints beside each method.
