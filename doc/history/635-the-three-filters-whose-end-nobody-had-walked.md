# 635 — The three filters whose end nobody had walked

`doc/todo/03` §22's other half. Sessions 631 and 633 gave §8.9.7's filtered inline image a derived
extent for four of the six filters Table 92 admits; `CCITTFaxDecode`, `DCTDecode` and
`RunLengthDecode` — nearly half the population between them — still had their end **searched for**.
Each of the three states its end in its own framing and **none of them needs a decoder** to find
it, which is one more than 633's owed list said.

Date: 2026-08-21.
ADR: [0467](../adr/0467-the-three-filters-whose-end-nobody-had-walked.md).

Touched: `crates/pdf-syntax/src/filter.rs` (`Delimiting`, `encoded_extent` split into
`decoded_extent`, `marker_extent`, `run_length_extent`, `jpeg_extent` with `next_marker`/
`entropy_end`/`Framing`, and `end_of_block_extent`), `crates/pdf-syntax/src/document.rs`
(`delimiting`, `Document::filtered_extent`), `crates/pdf-syntax/src/lib.rs`;
`crates/pdf-model/src/inline_image.rs` (answer 3 becomes one call; `terminating_marker` and `find`
deleted), `crates/pdf-model/tests/inline_images.rs` (nine tests and three fixture builders),
`crates/pdf-model/examples/token_window_census.rs` (per-filter verdicts, its own predicate, the
over-run diagnostic); `doc/conformance/ledger.toml` (§7.4.5, §7.4.6, §7.4.8, §7.3.8.2, §8.9.7),
`doc/checks/fixed-documents.toml` (one row), `doc/todo/03` (§22's bullet struck, §24 added), the
ADR and this file.

## What each clause makes derivable, per filter

- **§7.4.5, `RunLengthDecode`** — "each run shall consist of a length byte followed by 1 to 128
  bytes of data", and "[a] length value of 128 shall denote EOD". Every byte is a header or is
  counted by one, so the walk reads one byte in every two to a hundred and twenty-nine.
- **§7.4.8, `DCTDecode`** — the clause states the framing by reference to ISO/IEC 10918 and that
  standard supplies the EOI marker. **A search for `FFD9` would be wrong**: an `APPn` segment may
  carry an entire second JPEG, which is what a camera's thumbnail is, and its EOI stands first. So
  the walk steps over each segment by its stated length, and inside entropy-coded data leans on
  10918-1's byte stuffing, where the only `FF` pairs are `FF00` and the restart markers.
- **§7.4.6, `CCITTFaxDecode`** — Table 11's `/EndOfBlock`, whose default is true, makes an
  end-of-block pattern a requirement on the data; `/K`'s sign chooses between T.6's EOFB (two
  end-of-line codes) and T.4's RTC (six). **What makes finding it a reading rather than a guess** is
  T.4's construction of the end-of-line code so that no sequence of valid codewords can contain it —
  eleven zero bits and a one cannot stand inside encoded scan lines. `/EndOfBlock false` is where
  the clause itself puts the end outside the data, and that arm keeps the search.

All three are walks in `pdf-syntax`, which decodes no image codec and still does not: a walk over
lengths and markers reconstructs no sample. The module header says so rather than leaving a reader
to wonder why `DCTDecode` appears in a file whose first heading is "Only what can be done safely".

## The population, before and after

`examples/token_window_census`, extended to report **per filter** and to build its **own**
`Delimiting` from each image's dictionary rather than calling the code under test (trap 8). Run
whole before the change and whole after: 66 960 documents that open of 67 213, **926 308 pages**,
356 s and 286 s.

|  | before | after |
|---|---|---|
| filtered inline images with no `/L` | 2 672 351 | 2 672 782 |
| answerable | 2 671 901 | 2 672 332 |
| **ending early** | **1** | **0** |
| **over-running** | **1 229** | **0** |
| pages where the two answers disagree | 50 | 0 |

Per filter, before: `FlateDecode` 1 367 153, **`CCITTFaxDecode` 1 272 438**, `ASCII85Decode`
23 018, `ASCIIHexDecode` 4 302, `DCTDecode` 3 781, `RunLengthDecode` 1 655.

**Three things in that are the finding.**

**`CCITTFaxDecode` is a negative result and it is worth as much as the fix.** All 1 272 438 carry
Table 11's pattern and all 1 272 438 already agreed with the search. Half the population was being
guessed at *correctly*; no page moves, and what changes is that the answer is now derived. Trap 8
from the other side — a corpus cannot show you a rule it does not break.

**One image was ending early**: `7311536.pdf` page 9, `DCTDecode`, 163 bytes short of its EOI —
and it was already visible in this same census under another name. The largest single lexical
object over **8.88 billion** content tokens was a **798.20 KiB string on that page**: the 163 lost
bytes were handed to the lexer and one of them opened a string that ran to the end of the stream.
After the change the largest is 390.16 KiB on `219789.pdf` page 9, which is `doc/todo/14`'s road D
sizing its window against a real string instead of an artefact.

**The 1 229 over-runs are one byte each** (widest 1 B). §8.9.7 excludes "the white-space delimiting
those operators", singular, so a producer that writes two leaves one in the data and the derived
end does not. The census counts them apart from the early one and prints the widest, so a later
round need not take that sentence on trust.

## What it is pinned by, and trap 13

Nine tests, hand-built, each asserting its own premise first — three pairs (`RunLengthDecode`,
`DCTDecode`, `CCITTFaxDecode`), a Group 3 test that pins `/K`'s six end-of-lines against Group 4's
two, a window test that must answer `Truncated` rather than search, and the one that says why the
JPEG answer is a walk: a fixture with **two** EOI markers, the inner one inside an `APP1`
thumbnail.

Trap 13 run rather than assumed: with `Document::delimiting`'s three arms held at their pre-round
answers and nothing else changed, exactly the six defect-facing tests fail and all three twins pass.

## What moved

`7311536.pdf` page 9, this tree's own ink over its own raster: **1.617 with 8 array-operand reports
→ 54.406 with none**, against `pdftoppm` 57.99, `mutool` 57.61 and `gs` 62.44 through ImageMagick's
mean (a different formula, quoted as a neighbourhood). It is a row in
`doc/checks/fixed-documents.toml`, §20's rule, and the only gate that will see it at a merge.

## The ledger

Five rows, and one of them is a correction rather than an addition. §7.3.8.2's note closed with
"[t]wo of §7.4's filters can answer today and five cannot, for the same reason the pump names — a
resumable decoder". **A resumable decoder is what one of the five needed and none of the other four
did**, and that sentence had been standing on the instrument rather than on the clauses. Corrected
in place, with which clause answers each.

`spec-errata emit` over clauses 7 and 8 before anything was written, per `doc/errata-read.md`.
§7.3.8.2's Issue #319 is what decides the arithmetic in every arm — the encoded data "encompasses
all enveloping markers of the encoding, e.g. end-of-data markers" — and this is the first round
whose numbers rest on it. §8.9.7's Issue #20 was read and is **not** load-bearing: "its final or
only filter" is the outermost, which is the first named, which is what Table 5's application order
already made this code ask.

## The sequence

Run on a machine with three other rounds building, which is stated because `doc/todo/02` §2 says a
loaded machine is a silent third party to any gate that spawns a reference. The two that could
have been affected were run when the load average was in the low teens and both agree with the
merge round's figures exactly.

| | |
|---|---|
| `fmt`, `clippy --workspace --all-targets` under `-D warnings` | clean |
| `nextest --workspace` | **2342 passed, 17 skipped** |
| doctests, `-p conformance` | clean (157 + 5 + 1) |
| corpus | 974 documents, 68 incomplete |
| oracle | 1794 pages — 907 agrees, 66 contradicted, 13 not comparable |
| `render-quorra` | 957 pages — 932 agree, 23 differ, 2 refused |
| **`fixed_documents`** | **30 checked, 0 absent** |
| text extraction, both censuses, dates, XMP, JPEG 2000 | clean |
| `cargo deny check` | advisories, bans, licenses, sources ok |

§5's seven artefacts rebuilt and installed, which `tools/round.sh` reported missing at the start.

## Owed

- **A row in `doc/checks/fixed-documents.toml` is a page-9 row**, the first that is not page one.
  The harness already took it; nothing else in the file exercises that path.
- **Lead 2 of `doc/todo/37`** is still where 633 left it, and still wants a quiet machine.
