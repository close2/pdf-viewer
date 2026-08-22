# Session 653 — The page a matrix with no inverse cost

2026-08-22. Branch `round-653`, a parallel round beside 650, 651 and 652. `doc/todo/11` item 8:
a paint that cannot be positioned cost the page rather than the mark. ADR 0482.

## What the clause says

§8.3.4's third NOTE, which no code in this tree had cited:

> Not all transformations are invertible, however. For example, if a matrix contains a, b, c, and d
> elements that are all zero, all user coordinates map to the same device coordinates and there is
> no unique inverse transformation. Such noninvertible transformations are not very useful and
> generally arise from unintended operations, such as scaling by 0. Use of a noninvertible matrix
> when painting graphics objects can result in unpredictable behaviour.

Three readings: the mark has **no area** (a page transform is invertible, so a singular command
transform lands the whole path on a line or a point at every scale); the standard states nothing
further about it, and it is a NOTE besides; and it says nothing at all about the **page**, which
§6.3.2.2 asks for without an exception for one command. So the refusal is the mark's. ADR 0482 has
the argument and the §10.7.4 residual it does not take.

## Every site that refused

Item 8 named `render-cpu` and predicted `render-gpu`. Both were there and **there was a third**:

| backend | error | why |
|---|---|---|
| `render-cpu` | `CpuRasterError::UnsupportedPaint` | `page_to_path` inverted the command transform, before looking at the paint |
| `render-gpu` | `GpuRasterError::UnsupportedPaint` | `Spaces::new`, the same quantity |
| `render-quorra` | `Scene(InvalidStroke { width: 0.0 })` | `path_width × max_stretch`, which a collapsing transform makes zero |

`render-quorra` needs no inverse at all — it anchors a paint in page space — and refused anyway,
which is the evidence that the inverse was `tiny-skia`'s and Vello's requirement rather than the
clause's. It was found by the pair test rather than by reading: the fixture's first stroke scene
aborted the run.

All three now consult `pdf_render::paint_space(transform)`: page→path where there is one, and `None`
means *this mark is refused and the page is drawn*. `Command::Image` takes the same guard on all
three. `pdf_render::DisplayList::noninvertible_marks` counts them and
`pdf_model::interpret` raises `Unsupported::NoninvertibleMatrix { commands }`, worded for a reader
by `viewer_core::describe` — the twelfth place this program reports while drawing.

## The population

`crates/pdf-model/examples/singular_transform_census`, one process per archive.

```text
  the crawl, first pages
    65 944 files, 65 703 opened, 65 659 first pages read, 145 archives
    marks under a noninvertible matrix    389 on 13 pages of 13 documents
    of those, fills and strokes           102 in  5 documents   <- the pages that were lost
    of those, carrying a shading            0 in  0 documents

  those 13 documents, every page
    50 pages read: 422 marks on 46 pages of 13 documents; 102 fill/stroke in 5 documents

  doc/pdf.js, every page (the gated corpus)
    974 files, 964 opened, 1763 pages: 0 marks, 0 documents

  format-corpus, pdfbox, pdf20examples, pdf-differences, corpora-own, every page
    277 files, 275 opened, 2925 pages: 2 marks on 1 page of 1 document
    govdocs1-error-pdfs/error_set_2/150277.pdf page 105, 2 of its 521 commands
```

**Not one document anywhere carries a shading under such a matrix**, which is what the refusal was
argued for: `render-cpu`'s doc comment said "the alternative is a gradient placed somewhere
arbitrary". Every measured witness is a solid-coloured fill or stroke, and a solid colour is
positioned by nothing — so the entire measured population was pages refused for an inverse nobody
was going to read.

**Five crawled documents lost their whole page**, and four of the five are ordinary well-formed
files:

```text
  4605705.pdf  p1    97 of  282 commands   eight damaged Flate streams decoding into garbage
  2883540.pdf  p1     2 of  192 commands   a Finnish magazine media-kit cover
  0546320.pdf  p1     1 of 2582 commands   a Spanish water-utility application form
  1407697.pdf  p1     1 of 2891 commands   a US Letter page
  7803534.pdf  p1     1 of 1025 commands   a German course registration form
                       6972 commands between them
```

All five now render; all five are rows in `doc/checks/fixed-documents.toml`, which is **40 rows, 40
checked, 0 absent**. `2883540.pdf` was looked at beside `mutool draw` (trap 1) and the two pages are
the same picture; the others were looked at on their own.

## What pins it

`crates/render-quorra/tests/singular_transform.rs`, on all three backends: a page with a mark that
must survive beside a command under a singular matrix, and its twin without that command; the two
rasters must be byte-identical. Three matrices (four zero elements, one axis scaled by zero, rank
one with no zero entry), two paints, both painting operators, two scales one of them fractional.
A control asserts the same command under an *invertible* matrix marks the page, so the file cannot
pass on a backend that draws nothing at all.

Run against the defect first: with `render-cpu`'s `?` restored, `examples/render_at` aborts with
`UnsupportedPaint` on all four documents tried, and the pair test's first scene fails.

## Gates

Everything, because this reaches `pdf-render`, `pdf-model` and `render-cpu`. **Three neighbours
were building throughout; load average 15 to 21 for the whole sequence**, so every wall clock below
is a loaded one — ADR 0281's own caution, and the oracle's 98.8 s should be read against the 70.9 s
idle / 211.3 s loaded spread session 645 measured.

```text
  cargo fmt --all --check                    clean (after one `cargo fmt --all`)
  RUSTFLAGS="-D warnings" clippy --all-targets   clean
  cargo nextest run --workspace              2385 passed, 17 skipped, 39.7s
  cargo test --workspace --doc               ok
  fuzz --bins check                          ok
  corpus        974 documents in 5.5s: 0 unopenable, 8 locked, 2 encrypted beyond us,
                6 pageless, 68 incomplete, 0 slow
  oracle        908 agrees, 65 contradicted, 786 ambiguous, 2 our geometry,
                2 reference geometry, 13 not comparable, 18 no render; 98.8s
  text          10969/11163 words in bounds (98.26%), 486 of 508 documents fully in bounds
  selection     ok, 0 panicked
  accessibility ok, 0 untagged pages given structure, 0 panicked
  dates         1514 of 1545 conform (97.99%)
  xmp           ok
  jpeg2000      ok
  quorra        957 pages in 31.1s: 933 agree, 22 differ, 2 refused, 17 not comparable
  fixed_documents  40 checked, 0 absent, 40 rows
  conformance   5 + 1 tests ok
```

**No gated page can have moved and none did.** The census above reads 0 marks over every page of
`doc/pdf.js`, which is what every one of the corpus, oracle, text, selection, accessibility and
quorra gates walks, so the change is byte-identical there by construction rather than by
observation. `doc/todo/00` step 7's ink sweep is therefore not owed: the sweep measures our ink
against the lightest reference over the ambiguous pages, and no ambiguous page states the construct.

Clippy caught one thing worth recording: the guard pushed `render_quorra::stroke::encode` to 102
lines against `too_many_lines`'s 100, which is why the §8.3.4 test is folded into the `path.is_empty()`
early return there rather than standing beside it — the two are one sentence (*this command marks
nothing*) and the comment says so.

## One thing the witness taught

`4605705.pdf`'s singular matrix is of **full rank**: `a = 2.8064233e22`, `b = -4.296242e18`,
`c = -9.1778316e21`, `d = 1.4049977e18`, whose `a·d` and `b·c` agree to every bit an `f32` has, so
the determinant cancels to zero in the *arithmetic* rather than in the file. "Singular" is a property
of the computation there as much as of the document — a second reason not to build §10.7.4's run of
pixels for the collapsed case on the strength of that witness, since the geometry such a matrix
states is not recoverable from anything the renderer can compute about it. The four well-formed
witnesses are the ordinary kind, `a` or `d` exactly zero.

## What item 8 still owes

§10.7.4's own mark for a shape its **transform** collapsed. "[N]o shape ever disappears" would give
the collapsed image the run of whole device pixels it passes through, which
`pdf_render::split_collapsed_fill` builds for a subpath flat in its *own* space and nothing builds
for one flattened by its matrix. A round taking it owes a warrant (§8.3.4 NOTE 3 is the standard
declining to state the case, which is weaker than what `split_collapsed_fill` has), a *placement*
and not only an extent (a rotated collapse is a staircase; a rank-zero one is a single point, which
is §8.5.3.3.1's case), and a witness — of which there is none. `doc/todo/11` item 8 carries all
three.
