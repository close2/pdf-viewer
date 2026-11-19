# RFC 0016 — A portable conformance suite: expected outcomes from the clause alone

Status: **proposed**, with a first slice built — three questions in section 10; `doc/questions/Q948`
asks them. Nothing here changes a ledger status or a line of code that ships. The slice adds a
top-level directory, `portable-conformance/`, a workspace binary that answers it for this program
(`tools/portable-adapter`), and one conformance test (`tools/conformance/tests/portable_quotes.rs`).
Round: 2126. The owner, 2026-11-19: after the ledger and the clause 13 plan, "start audits and a
portable conformance test suite (still in parallel to the other tasks)".
Builds on: CLAUDE.md principle 5, ADRs 0187 (how the standard's text is held) and 0258 (the
corpora's licences), `doc/third-party-data.md`, `doc/gwg-output-suite-provenance.md`.
Decides: ADRs 3205 to 3208. Asks: **Q948** (section 10).

`§N` is a clause of ISO 32000-2:2020 and nothing else. Figures marked *measured* were run on
2026-11-19 on this machine. Figures marked *fetched* were read that day from the public sources in
section 12.

## 0. In one paragraph

The ledger holds 788 `implemented` rows, and each is held by fixture tests written in Rust against
this tree's own APIs. No other processor can run them, and they cannot show that a reading of a
clause is right, only that this tree agrees with itself. A portable suite fixes that. Each test is
a PDF built from readable source. Each states the clause sentence it rests on and an expected
outcome **derived from that sentence and nothing else**: probe points that must or must not be
marked, an equality the clause states, a string the clause's mapping gives, or a one-line answer
to a question the clause decides. Any processor runs it through a small executable adapter. The
first slice has 17 tests on 14 clauses, with 40 checks across the four kinds. Under this program all 17 pass. Under
poppler, MuPDF and Ghostscript every test they can be asked passes, except two under Ghostscript
10.05.1. In both of those, the clause's text decides against Ghostscript (section 7). Every check was
calibrated by planting one defect per test, and every planted defect was reported (section 8).

## 1. What this round was asked, and what it does not do

Asked: the precedents and what each lacks; what an expected outcome can be, by kind; the format, the
licence and versioning; the runner and its adapters; where the suite lives and how it ties to the
ledger; a first slice of 10 to 20 tests, built and run; questions for the owner.

Not done: the ledger tie is designed (section 6) and not built, because its form depends on where the
suite lives (Q948 question 1). No pdf.js adapter: this machine has no JavaScript runtime, and the
owner installs system packages (section 5). No test needs a binary resource (a font program, an
image), and section 4.3 says how one would be held.

## 2. Precedents, and what each lacks

| suite | what it holds | expected outcome | why it is not this |
|---|---|---|---|
| PDF Association `pdf20examples` (`doc/corpora/pdf20examples`, CC BY-SA 4.0) | Example PDF 2.0 files: a simple file, UTF-8 strings, an incremental save, a page-level output intent | none stated | A file with no expected outcome tests only that a processor opens it |
| PDF Association `pdf-differences` (`doc/corpora/pdf-differences`, PDFs CC BY 4.0) | Minimal hand-written files, each on an issue "seen across multiple widely-used implementations", with a README per case | prose and screenshots; the README warns that screenshots "should **not** be compared at a pixel level" | The closest precedent in spirit (readable files, one issue each). Its verdict is a person reading prose, so nothing runs it |
| veraPDF corpus, with Isartor and the BFO PDF/A-2 suite (*fetched*, CC BY 4.0) | Atomic files for PDF/A 1 to 4, PDF/UA 1 and 2, and some of ISO 32000-1 and -2, "self-documented via the document outlines", named by the part and clause they test | pass or fail of a validation rule | It asks whether a file *conforms*. This suite asks whether a *processor* reads a conforming file as the clause says. The two questions share no expected value |
| Ghent PDF Output Suite 5.0 (fetched by `tools/fetch-gwg.sh`, never tracked: its licence forbids redistribution) | Print-production patches: overprint, separations, blend modes, font types | a visual rule built into each patch (a faint `X` passes, a clear one fails), judged by eye | Not redistributable, judged by a person, and written for a press. `doc/gwg-output-suite.md` holds this tree's verdicts |
| pdf.js reference tests (`test/test_manifest.json`, *fetched*) | 1551 entries: 1282 `eq`, 85 `text`, 58 `load`, 84 `other`, the rest `extract`, `fbf` and `highlight`; 527 are `link` entries whose PDFs are downloaded from elsewhere | for `eq`, a snapshot pdf.js itself drew (`makeref`) | Regression testing: it asks whether pdf.js still draws what it drew before. Run against another processor, it would measure distance from pdf.js |

What none of them has, and what this suite is for: **an expected value a machine checks, derived from
the standard rather than from a processor, for any processor to run.** Trap 9 applies to each of the
precedents just as it applies to this tree. Two references can agree because they share code or share
a gap. A reference image cannot say which of its pixels the standard fixed.

## 3. What an expected outcome can be, by kind

**(a) Pixels.** Never a reference image (ADR 3206). Three forms are analytic:

- **A region the clause fixes**, probed inside and outside. The probe is black paint on white paper
  (§10.4.2.2: "0.0 corresponds to black, 1.0 to white"). It sits at least five device pixels (2.5
  units at 144 dpi) from every edge the clause fixes, so anti-aliasing, the scan-conversion latitude
  §10.7 leaves the device, and stroke adjustment cannot reach it. The classification is a quarter of
  the range from each end: *marked* means every component is at most 64, *unmarked* at least 191.
  That is wide enough for a processor whose black is not exactly 0 (an output profile's black point
  under §10.3), and still narrow enough that a wrong shape fails. Each test also puts a probe
  **where the clause's alternatives differ**. The round cap's test probes (24, 34), 2.6 units
  outside the arc, which a projecting square cap would mark.
- **An equality the clause states.** §10.4.2.2: "A gray level shall be equivalent to an RGB value
  with all three components the same." §10.4.2.1 lets a processor convert through §10.3 instead of
  the classic formula. So the test checks that the three components are equal, within 2 of 255 for
  two independent roundings to 8 bits, and that the level lies strictly between the two thresholds.
  It does not check *which* level, because the clause does not fix it.
- **A size**, only where the page's extent in pixels is a whole number. A fractional extent becomes
  pixels by a rule the standard does not state (`pdf_transform::render::Overrun` reports this
  tree's choice).

**(b) Text.** The exact string §9.10's mapping gives, with leading and trailing white space ignored.
§9.10 maps codes to characters. It says nothing of the spaces and line breaks an extractor puts
around a run, so those are not checked. §9.10.2 says a processor "can use these methods, in the priority
given". A processor that maps codes by some other route is reported as disagreeing, and the
manifest's `why` names the method its value comes from.

**(c) Structure.** A one-line answer to a question a clause decides: the page a named destination
designates (§12.3.2.4), the page label of an index (§12.4.2), the value of a field by its fully
qualified name (§12.7.4). The vocabulary is small on purpose. Each question added is a request every
adapter must answer or decline, so a question enters only when a clause needs it.

**(d) Refusal, or recovery a clause states.** Most malformations the standard says nothing about,
and those belong to the robustness question (the corpus and the oracle), not here. A test enters
only where the standard states the outcome. §7.3.10 says "[a]n indirect reference to an undefined
object shall not be considered an error", and §7.3.7 says a null entry "shall be treated the same
as if the entry does not exist". Each outcome is checked through (a) to (c): the page opens,
unrotated, at its size, or turned by the `/Rotate` it inherits past its own null. The standard
states few rejections outright. A test that a file "must be refused" needs a sentence that says so,
and the slice found none it needed.

**(e) What the standard leaves open.** No test, and this section names the cases so that nobody
writes one later: anti-aliasing and the scan conversion §10.7 leaves the device (§10.7.2 permits a
processor to ignore flatness, and §10.7.3 says each device "may have internal limits" on smoothness);
how a fractional page becomes whole pixels; the level a converted colour takes under §10.3's
profiles; the artwork of §12.5.6.4's icons; the appearance of a substituted font for an unembedded
one (§9.6.2.2 names the standard 14 by their metrics, not their outlines); the spacing an extractor
inserts between runs. A test of any of these would encode one processor's choice as the standard.

## 4. Format

### 4.1 A manifest is the test

One TOML file per test under `portable-conformance/tests/`, named `<clause>-<what>.toml`
(`portable-conformance/README.md` gives the fields). It holds the clause, the verbatim sentences, the
kind, the file as readable PDF source and the checks. Every check has a `why`: the arithmetic or the
sentence behind its value and tolerance. In the file, each indirect object's `body` is PDF syntax
written by hand, and each stream's data sits beside it. A reviewer reads the PDF, the clause and the
expectation on one screen.

### 4.2 The generator, and why no PDF is committed

`portable-conformance/tools/generate.py` adds the header with its binary comment, every `/Length`,
the cross-reference table's 20-byte entries and their offsets, and the trailer. It adds nothing
else. Its output is a pure function of the manifest, and `--check` verifies a directory against it
(*measured*: identical bytes on a second run; `qpdf --check` finds no syntax error in any of the 17
files). A hand-written offset is the commonest error in a hand-written PDF, and a wrong one turns the
file into a test of §7.5.4's recovery rather than of its clause (ADR 3205).

### 4.3 What cannot be written by hand

A font program, an image, or a compressed stream cannot be a readable body. The proposal: a manifest
names such a resource as `source` plus `recipe`. The source is a text file in the suite (an SVG, a
`ttx` dump, a PPM written as numbers), and the recipe is the one standard-library step that makes it
bytes (`zlib.compress` for `/FlateDecode`, an ASCII-hex filter that needs no step at all). The suite
never holds a binary without its source. Not built: the slice needed none.

### 4.4 Licence

- **Manifests and generated PDFs: CC0 1.0.** A test file is most useful copied into another
  project's test directory. The veraPDF corpus and `pdf-differences` use CC BY 4.0, under which every
  copy carries an attribution forward. CC0 removes that friction, and nothing in a three-line content
  stream needs protecting.
- **Generator, runner and adapters: Apache-2.0**, this repository's licence.
- **The quotations are ISO's.** Each is a sentence or two, attributed to its clause of ISO
  32000-2:2020, quoted as citation in the same way as the sentences in this tree's source comments.
  The standard itself is held as ADR 0187 holds it: `doc/md/` is ignored, so the text is not in any
  clone, and is encrypted in `doc/specifications.zip` for the gates. The suite neither ships the text
  nor needs it to run. The conformance gate checks every quotation against `doc/md/` here (ADR 3208).
  It found one on its first run: a §7.3.10 manifest quoting §7.3.7's sentence.

### 4.5 Versioning

`portable-conformance/suite.toml` holds a version and the standard's edition. A new test is a patch
release; a changed expected value or a new check type is a minor release and names its clause; a
new edition of the standard is a major release. A result is reported against a suite version.

## 5. The runner and its adapters

`portable-conformance/tools/run.py` is Python's standard library alone, including a PNG reader of
about 60 lines. It runs every manifest against each adapter and prints one table row per test,
ordered by clause, then each verdict that is not a pass with its detail. `--json` keeps everything.

An adapter is any executable (ADR 3207):

```
<adapter> render <pdf> <page> <dpi> <out.png>
<adapter> text   <pdf> <page>
<adapter> query  <pdf> <question ...>
```

Exit 3 means the processor cannot be asked, and the table shows `n/a`. That keeps "cannot" apart
from "wrong": poppler's programs print no page label, and a failure would have charged poppler with a
reading it never offered.

| adapter | render | text | query | role |
|---|---|---|---|---|
| `quorra` | `quorra-transform render` (the CPU oracle, `render-cpu`) | `portable-adapter text` | `portable-adapter query`: `page-count`, `page-label`, `destination`, `field-value` | this program |
| `poppler` | `pdftoppm` | `pdftotext` | `destination` by `pdfinfo -dests` | evidence |
| `mupdf` | `mutool draw` | `mutool draw -F txt` | none | evidence |
| `ghostscript` | `gs -sDEVICE=png16m` | `gs -sDEVICE=txtwrite` | none | evidence |

`portable-adapter` (`tools/portable-adapter`) is one `pdf_model` call per request, the call the viewer
makes, so a verdict is about the program a person runs. Pages are drawn by `quorra-transform`, the
shipped command, rather than by a harness of its own. **What the `quorra` adapter does not yet
cover**: the product draws page one on the device (CLAUDE.md principle 2). A `quorra-gpu` adapter
needs a command that draws a page to a file through `render-raster`. No shipped command does, and
Q948 question 3 asks whether to add one.

pdf.js needs a JavaScript runtime, and this machine has none. The adapter is about twenty lines of
Node around `pdfjs-dist` and its `node-canvas` peer. The owner installs system packages, so it waits
on Q948 question 3.

## 6. Where it lives, and how it ties to the ledger

**Now: a top-level directory, `portable-conformance/`** (ADR 3205). It is not in `crates/` or
`tools/`, because it is not part of this program. It must make sense to a processor that has never
seen this tree, and moving it out later is a `git subtree split` of one directory. **Recommended
later: its own repository** under the owner's account, once it holds enough tests to be worth
another project's time (a hundred is a reasonable mark). At that point this tree pins a version of
it, in the same way that `doc/corpora/` pins submodules. Moving now would split review of the slice
from the code it tests while the format is still settling.

**The ledger tie, designed and not built:**

- A ledger row gains an optional `portable = ["<id>", ...]`, naming the manifests that test it.
- A conformance test holds each named id to an existing manifest whose `clause` is the row's clause
  or a subclause of it. It also holds each manifest to at least one row that names it, so a
  manifest cannot drift away from the ledger.
- `tools/state.sh` gains a count of `implemented` rows with a portable test. That count is the
  second denominator of section 9. It is printed by a command, never written down (ADR 0281).
- If the suite moves out, the tie names `<version>/<id>` and the test reads the pinned copy.

## 7. The first slice, built and run

17 tests and 40 checks: 9 pixels, 3 text, 3 structure and 2 refusal. They cover §7.3.7, §7.3.10, §8.4.3.3 (×3), §8.4.4, §8.5.3.3.2,
§8.5.3.3.3, §8.5.4, §9.10.2 (×2), §9.10.3, §10.4.2.2, §12.3.2.4, §12.4.2, §12.5.5 and §12.7.4.1.
Each of those rows is `implemented` in the ledger except §12.5.5, which is `departed`. That row's
departure is not about selecting by `/AS`. Two tests run the clause's own EXAMPLE: §12.4.2's page
labels "i, ii, iii, iv, 1, 2, 3, A-8, A-9", and §9.10.3's ToUnicode CMap with its ligature array and
its code past the Basic Multilingual Plane.

*Measured* (`run.py` with all four adapters; this program at the round's head):

| clause | test | kind | quorra | poppler | mupdf | ghostscript |
|---|---|---|---|---|---|---|
| 7.3.7 | null-entry-is-absent | refusal | pass | pass | pass | **fail** |
| 7.3.10 | undefined-reference-is-null | refusal | pass | pass | pass | pass |
| 8.4.3.3 | butt-cap | pixels | pass | pass | pass | pass |
| 8.4.3.3 | projecting-square-cap | pixels | pass | pass | pass | pass |
| 8.4.3.3 | round-cap | pixels | pass | pass | pass | pass |
| 8.4.4 | cm | pixels | pass | pass | pass | pass |
| 8.5.3.3.2 | non-zero | pixels | pass | pass | pass | pass |
| 8.5.3.3.3 | even-odd | pixels | pass | pass | pass | pass |
| 8.5.4 | clip | pixels | pass | pass | pass | pass |
| 9.10.2 | glyph-name-from-differences | text | pass | pass | pass | pass |
| 9.10.2 | tounicode-first | text | pass | pass | pass | pass |
| 9.10.3 | example-ligatures-and-supplementary | text | pass | pass | pass | **fail** |
| 10.4.2.2 | grey-is-neutral | pixels | pass | pass | pass | pass |
| 12.3.2.4 | named-destination-in-dests | structure | pass | pass | n/a | n/a |
| 12.4.2 | page-labels-example | structure | pass | n/a | n/a | n/a |
| 12.5.5 | appearance-state | pixels | pass | pass | pass | pass |
| 12.7.4.1 | inherited-field-value | structure | pass | n/a | n/a | n/a |

Totals: quorra 17 of 17; poppler 15 of 17 with 2 `n/a`; mupdf 14 of 17 with 3 `n/a`; Ghostscript
10.05.1 12 of 17, with 2 failures and 3 `n/a`.

**The two disagreements, taken back to the clause** (principle 5):

- **§7.3.7, Ghostscript.** The page tree node says `/Rotate 90` and the page says `/Rotate null`.
  Ghostscript draws the page upright, 200 by 400 pixels. A control file with the page's entry removed
  is drawn turned, which shows that Ghostscript honours inheritance and reads `null` as a value. The
  clause says a null entry "shall be treated the same as if the entry does not exist", and Table 31
  makes `Rotate` inheritable. **Our reading holds.**
- **§9.10.3, Ghostscript.** The CMap maps `<3A51>` to `<D840DC3E>`, which the clause names as
  U+2003E "expressed by the byte sequence <D840DC3E> in UTF-16BE encoding". Ghostscript's
  `txtwrite` emits `ed a1 80 ed b0 be`, each UTF-16 code unit written as its own three bytes. That is
  not UTF-8 for U+2003E, so the pair was never combined. **Our reading holds.**

This program's grey test drew (128, 128, 128). That is the classic formula, and the test does not
require it.

## 8. Calibration: every check fails when its clause is broken

Trap 13. A suite that passes everything has shown nothing until it is seen to fail. Each of the 17
manifests was copied and given one defect: the cap style changed, `f` and `f*` swapped, the clip
removed, the `cm` removed, the grey made chromatic, `/AS /On` made `/Off`, the undefined reference
made `/Rotate 90`, the null made `0`, the ToUnicode target changed, `/eacute` made `/egrave`, the
surrogate's last byte changed, the destination moved, `/St 8` made `9`, the inherited value changed.
*Measured*: **this program failed all 17 planted tests**, each failure the planted defect (for example
"page-label 7: 'A-9', expected 'A-8'"). poppler and MuPDF failed every planted test they could be
asked. ADR 3208's quotation gate has its own planted control in its test file.

## 9. Cost, order and what comes next

The slice cost about 700 lines: the generator, the runner, four adapters, the Rust adapter and the
gate test, plus about 30 lines of manifest per test. A next slice is one manifest per clause, about a
quarter of an hour each where the clause fixes a region, an equality or a string. How many of the
788 `implemented` rows carry a checkable expectation of kinds (a) to (d) is not counted. The rest
are (e), or are requirements on writers, which a processor's output cannot show. The ledger tie's
count (section 6) will measure it rather than estimate it. Recommended order:

1. The ledger tie (section 6), once Q948 question 1 is answered, so that each new test lands on its row.
2. Clause 8's remaining operators and graphics state, where most expectations are regions.
3. §9.10 and Annex D's encodings: strings, exact and cheap.
4. Clause 12's structure questions: outlines, actions, optional content's default configuration as
   pixels (§8.11.4.3).
5. Clause 11 where a clause fixes an arithmetic result. A `Normal` blend of two opaque colours is a
   region. A blend mode's formula fixes an equality only up to §10.3's conversion, so its checks are
   of the `neutral` shape rather than a level.

## 10. Questions for the owner — Q948

1. **Where does the suite live?** Recommendation: the top-level `portable-conformance/` now, and its
   own repository once it holds about a hundred tests, pinned by version here. Meanwhile it stays in
   this tree, and the ledger tie (section 6) is built against the in-tree directory.
2. **Which licence?** Recommendation: CC0 1.0 for the manifests and the PDFs they generate,
   Apache-2.0 for the tools, and ISO's sentences quoted as citation as the tree's comments already
   quote them. The alternative is CC BY 4.0, as veraPDF and `pdf-differences` use. Meanwhile the
   suite carries CC0's SPDX identifier in `suite.toml` and the README, and nothing is published
   outside this repository.
3. **How far do the adapters go?** Recommendation: this program's CPU adapter now. A `quorra-gpu`
   adapter needs a shipped command that draws a page through `render-raster`, and since the product
   draws page one on the device, the device's readings should be judged too. poppler, MuPDF and
   Ghostscript stay as evidence. pdf.js waits until a Node runtime is installed (an owner's system
   package). Meanwhile the four built adapters stand, and the evidence adapters never change an
   expected value.

## 11. The current restrictions, and the unconstrained design

The RFC process asks for the restrictions to be named rather than designed around. One binds here.
Principle 5 makes the standard the only source of an expected value, so this design has no
reference images, no "matches Acrobat", and no corpus-derived tolerance. That is not a restriction to
lift; it is what makes the suite worth running. Without it, the design would be pdf.js's: snapshots
of one renderer's output, with a regression test passed off as a conformance test.

## 12. Sources

- ISO 32000-2:2020 with Errata Collection 3, `doc/md/ISO_32000-2_sponsored_EC3.md` (held per ADR
  0187): §§7.3.7, 7.3.10, 8.4.3.3, 8.4.4, 8.5.2.1, 8.5.3.3.2, 8.5.3.3.3, 8.5.4, 9.10.2, 9.10.3,
  10.4.2.1, 10.4.2.2, 10.7, 12.3.2.4, 12.4.2, 12.5.5, 12.7.4.1.
- `doc/corpora/pdf20examples/README.md`, `doc/corpora/pdf-differences/README.md`;
  `doc/third-party-data.md`'s corpus table.
- veraPDF corpus README (`https://github.com/veraPDF/veraPDF-corpus`, branch `staging`), and its
  top-level directories, *fetched* 2026-11-19.
- pdf.js `test/test_manifest.json` (`https://github.com/mozilla/pdf.js`, `master`), *fetched*
  2026-11-19 and counted by `"type"` and `"link"`.
- `tools/fetch-gwg.sh`, `doc/gwg-output-suite.md`, `doc/gwg-output-suite-provenance.md`.
- poppler (`pdftoppm`, `pdftotext`, `pdfinfo`), MuPDF (`mutool`), Ghostscript 10.05.1, as Debian
  13 installs them on this machine.
