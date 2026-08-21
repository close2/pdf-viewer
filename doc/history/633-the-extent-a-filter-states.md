# 633 — The extent a filter states, and the search that was guessing at it

`doc/todo/03` §21's head, `7926872.pdf`, left diagnosed and unfixed: an inline image whose data is
filtered and which states no `/L` had the end of its data searched for rather than derived, and
§7.3.8.2 makes a filtered extent derivable from the filter's own end-of-data marker. Two crawled
photographs come back whole; a claim about a silence in the standard, six hundred sessions old, is
corrected by an erratum found on the way.

Date: 2026-08-21.
ADR: [0466](../adr/0466-the-extent-a-filter-states-and-a-search-was-guessing.md).

Touched: `crates/pdf-syntax/src/filter.rs` (`encoded_extent`, `EncodedExtent`, `Engine::new`,
`Engine::pump`, `Engine::consumed` factored out of `Pump`), `crates/pdf-syntax/src/document.rs`
(`Document::filtered_extent`), `crates/pdf-syntax/src/lib.rs`;
`crates/pdf-model/src/inline_image.rs` (answer 3, `Terminator`, the `expand_key` correction),
`crates/pdf-model/tests/inline_images.rs` (three tests and `flate_stored`),
`crates/pdf-model/examples/token_window_census.rs` (the population instrument);
`doc/conformance/ledger.toml` (§8.9.7, §7.3.8.2), `doc/checks/fixed-documents.toml` (two rows),
`doc/todo/03` (§21 struck, §22 added), `doc/todo/14` (a stale claim about the search),
`doc/todo/37` (the lead narrowed), the ADR and this file.

## What it was

`7926872.pdf` page one is one command — a 1200×1790 `/DeviceRGB` photograph written inline under
`FlateDecode` with no `/L` — and it drew 477 217 samples of 6 444 000 while reporting a short image
and a stream of "inside an array, which §7.3.6 admits only objects into" complaints, because 1.4 MB
of the photograph was being tokenised as content operators. The first `EI` that stands as its own
token is 24 822 bytes into 2.9 MB of compressed data.

§8.9.7 sends the reader to §7.3.8, and §7.3.8.2 says that "most filters are defined so that the
data shall be self-limiting; that is, they use an encoding scheme in which an explicit end-of-data
(EOD) marker delimits the extent of the data". `pdf_syntax::Pump` has counted its consumed input on
both its engines since ADR 0365 and exposed it to nobody; that was the whole distance between the
guess and the answer.

## The order of the work

**The population came first** (trap 11), and it is the part worth keeping. `token_window_census`
already sorted every inline image by which of §8.9.7's answers decides its extent; it gained one
comparison — ask the first filter of the chain where its marker stands, compare with where the scan
stopped — and was run over the crawl and the curated corpora *before* anything changed. **17 images
in 5 documents of 65 967 end early, costing 13.45 MiB; the curated corpora carry none at all.** So
nothing the corpus, oracle, quorra or text gates walk could ever have shown this, which is
`CLAUDE.md`'s two denominators in one line, and it is why the two documents went into
`doc/checks/fixed-documents.toml` rather than into a gate.

The census re-run after the change reports **0 early of 1 366 702**. Its totals move by 75 images
of 3 977 492, which the defect itself explains: where an image ends decides what the rest of the
content stream lexes as, so the two runs' denominators are not quite the same object.

The first run says how much is left: the first filter of the 2 672 062 filtered inline images with
no `/L` is `CCITTFaxDecode` **1 272 430** times against `FlateDecode`'s 1 367 073. Half the
population still has its end searched for, and that half is now a number in `doc/todo/03` §22
rather than a thing nobody had counted.

**Then the pair** (trap 8), and the mechanism is what makes it honest: `flate_stored` writes RFC
1951's *stored* block, which carries its payload literally, so the compressed bytes really do
contain ` EI ` and the test asserts that they do before asserting anything else. Its twin is the
same construction with no `EI` anywhere, where both answers agree — without it the pair would say
only that something changed. A third test cuts the buffer two bytes short of the data's end and
requires `Truncated` rather than a search, which is what makes the answer survive a window.

**Then trap 13**: with `inline_image.rs` reverted and nothing else, the first and third fail and the
twin passes.

## What moved

|  | before | after | `pdftoppm` / `mutool` / `gs` |
|---|---|---|---|
| `7926872.pdf` p1 | 2.915 | 44.516 | 45.233 / 45.020 / 44.647 |
| `4605499.pdf` p1 | 8.848 | 71.775 | 72.682 / 72.409 / 72.062 |

`4605499.pdf` is a second document the census found and §21's ranking never saw — its archive was
in none of the ten that chunk took — and at −63.2 it is deeper than the row this round was sent
after. Both report nothing now.

## The erratum

`doc/errata-read.md`'s rule is that a round implementing a clause runs `spec-errata emit` on that
document *before* it writes, and it paid on a clause this round had no intention of touching.
`expand_key`'s comment said that the standard states no rule for a file writing both an abbreviated
key and its full name, "and this is therefore a decision rather than a reading". Errata Collection
3, §8.9.7, Issue #3, states the rule outright, and it is the rule this tree already follows —
arrived at from `issue14256.pdf`'s bytes and recorded as a choice since the eleventh session. The
code did not move and the comment did. `check` could never have found it: it compares quotations
the tree has written, and nobody had written a quotation of a sentence that only exists in a caret.

## Lead 2, not taken, and why

`doc/todo/37`'s remaining lead — whether a rendering that lands while the view moves is ever
stranded on the *device* path — was read statically and left open. `Stale::plan` can refuse a
device-path frame in exactly two ways once a rendering exists: `InsideTheRefresh`, which bounds its
own wait to one refresh, and a base error with no retained coverage, which is this file's own
stated policy about a page turn. `NothingRendered` and `TooDear` are unreachable there. That
narrows the trace's target from an enum to two names — but it is an argument, not a trace, and the
refusal in question is a comparison of two measured durations on a machine that had four rounds
building on it. Taking the trace then would have measured the load, which is what defeated sessions
626 and 627.
