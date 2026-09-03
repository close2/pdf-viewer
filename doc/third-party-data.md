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
`include_str!`d by `pdf-viewer --licences`, put over the page by `?` as the About panel, set in
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
