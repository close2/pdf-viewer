# 993 — A date is what the working draft defines, and the corpus writes two of its forms

Date: 2026-09-12. Branch `batch-992-997`, from `1ea10797`. Stream: the PDF/A validator,
`crates/pdf-archive`. Five sibling rounds were live in the same worktree throughout (992 in
`crates/pdf-transform`, 994 in `CLAUDE.md`, `doc/rfc/` and `doc/questions/`, 995 in `doc/todo/02`
and `tools/`, 996 and 997 in parts of `crates/pdf-model`), none of them in `crates/pdf-archive/`.
A previous attempt at this round was cut off by a session limit before it did anything.

## What the round was asked to do, and what it did

Act on `doc/questions/A53`: widen `Lexical::Date` from the XMP Specification's six profiles to
the ISO 8601 forms the working draft the owner obtained defines, from that text alone, keeping
the `GPSCoordinate` check; measure before and after over every XMP packet the corpora hold;
calibrate both ways; and, for `A60`, price the part-3 adaptation in the code's own terms.

- **`crates/pdf-archive/src/iso_8601.rs` is new**: the grammar of ISO/WD 8601-1's clauses 4.1
  to 4.3 in this crate's words under the draft's clause numbers, with the element ranges from
  3.2's calendars — the leap rule, Table 1's month lengths, the week count from 2.2.10 and 3.2.2's
  reference point — and a `Refusal` that names the form a value missed. `Lexical::refused`
  replaces `accepts` so that the finding prints that phrase after the value. ADR 1013 sections
  2 and 3 have the three readings the text needed.
- **`examples/dates.rs` is new**: a census of every scalar of every `Date`-typed property in
  every metadata stream of a corpus, by form in the draft's notation, with every refusal listed.
  `pdf_archive::dates_stated` is the public accessor it needed.
- **`GPSCoordinate` stays**, on the answer's first sentence; the `Lexical` header says so.
- **`coverage.rs` is untouched**: no `says` there cites the date type, and
  `cargo run -q -p pdf-archive --example frontier` prints `none` before and after.
- **`doc/third-party-data.md`** carries the working draft's row: a WD, copyright ISO, reproduction
  to participants in the standards process only, so cited by clause and never quoted.

## Requirement identifiers

**None added, none removed, none re-keyed, none promoted.** The row that changed is
`metadata/properties-use-known-schemas`, already `Check::Implemented`, whose date predicate
admits more. `crates/pdf-transform/src/archive/decision.rs` was not touched (round 992's).

## The figures, off the run

`cargo run --release -p pdf-archive --example dates`, before and after, over the two tracked
corpora (`doc/pdf.js/test/pdfs` and `doc/veraPDF-corpus`, symbolic links followed): 3882 files,
3872 opened, 3271 metadata streams, 7404 date values of which 6 empty.

| | before | after |
|---|---|---|
| refused | 38 | 27 |
| admitted, the 6 empty values aside | 7360 | 7371 |

The eleven that moved, all from refused to admitted and none the other way:

| value | files | form, WD clause |
|---|---|---|
| `YYYY-MM-DDThh:mm:ss` (local time) | `ZapfDingbats.pdf`, `issue12120_reduced.pdf`, `issue18032.pdf`, `issue20232.pdf`, `issue20453.pdf` (`xmp:CreateDate`) | 4.3.2's empty zone designator |
| `YYYY-MM-DDThh:mm:ss.sss+hh` | `6-2-4-1-t01-pass-a`, `6-2-4-1-t01-pass-b` (PDF/A-4e), `7.1-t06-fail-a` (PDF/UA-1); `xmp:CreateDate` and `xmp:ModifyDate` each | 4.2.5.1's `±hh` |

The 27 still refused, each with its phrase in the census's output: seven `D:`-prefixed §7.9.4
dates (`issue9949.pdf` ×3, `6-1-5-t02-pass-a` ×2, `7.2-t27-fail-b` ×2), three
`…T14:02:0300:00` (`issue8702.pdf` ×3), four `…Z+01:00`, two `…TIME13…`, and eleven labelled
`Date: …`/`Time: …`/`DTO: …`/`GPS: …` — the last three groups all veraPDF *fail* witnesses for
section 6.6.2.3.1 and section 6.7.2, which still agree.

Admitted forms after, tracked corpora (the six `doc/corpora/` figures follow):

| count | form |
|---|---|
| 6924 | `YYYY-MM-DDThh:mm:ss±hh:mm` |
| 378 | `YYYY-MM-DDThh:mm:ssZ` |
| 36 | `YYYY-MM-DDThh:mm±hh:mm` |
| 6 | `YYYY-MM-DDThh:mm:ss.sss±hh` |
| 6 | `YYYY-MM-DDThh:mmZ` |
| 5 | `YYYY-MM-DD` |
| 5 | `YYYY-MM-DDThh:mm:ss` |
| 3 | `YYYY-MM-DDThh:mm:ss.sssssssss±hh:mm` |
| 2 each | `YYYY`, `YYYY-MM`, `YYYY-MM-DDThh:mm:ss.ssZ`, `YYYY-MM-DDThh:mm:ss.ss±hh:mm` |

Over `doc/corpora/format-corpus`, `pdf20examples`, `pdfbox` and `pdf-differences`: 275 files, 273
opened, 177 streams, 503 values, **0 refused before and after** — 430 `…±hh:mm`, 63 `…Z`, 5
`YYYY-MM-DDThh:mm±hh:mm`, 4 `YYYY-MM-DDThh:mm:ss.sssZ`, 1 `YYYY-MM-DDThh:mmZ`.

No value in either population exercised a range refusal (a month past twelve, a day the month
does not have, an hour of 24). No producer writes a basic-format, ordinal, week, expanded or
comma-fraction form.

The coverage census (`--example targets`) is byte for byte session 989's. The corpus harness
(`tools/bounded.sh --data 12 --tree 12 -- cargo test --profile gates -p pdf-archive --test corpus
-- --ignored --nocapture`), exit 0, 13 s, peak 0.69 GiB:

| target | agreed | missed | over | settled | elsewhere | unreadable |
|---|---|---|---|---|---|---|
| PDF/A-4 | 473 | 0 | 0 | 8 | 6 | 0 |
| PDF/A-4f | 9 | 0 | 0 | 0 | 2 | 0 |
| PDF/A-4e | 17 | 0 | 0 | 1 | 1 | 0 |
| PDF/A-2b | 971 | 1 | 0 | 7 | 7 | 0 |
| PDF/A-2u | 21 | 0 | 0 | 1 | 0 | 0 |
| PDF/A-2a | 27 | 0 | 0 | 0 | 0 | 0 |

`over` is 0 on all six, before and after; the one miss is the standing `6-6-2-3-3-t03-fail-b`
(errata A029). **Every row is identical to session 989's**, and ADR 1013 section 5 says why that
is the two denominators rather than a widening that did not bite: the three files whose values
moved are held to PDF/A-4e, where the schema row does not bind, or to PDF/UA-1.

The XMP gate (`cargo test --profile gates -p pdf-model --test xmp -- --ignored --nocapture`),
exit 0: 319 documents carry the stream, 318 read, 1 refused (`PDFBOX-3148-2-fuzzed.pdf`), 3191
properties — unchanged; this round reads `pdf_model::xmp` and edits nothing in it.

## Calibration

Both ways, in `crate::iso_8601`'s tests and `table::metadata`'s
`a_date_is_any_form_the_working_draft_defines_and_nothing_else`:

- a form the WD defines that the old check refused: `20260910` (`Q53`'s own example),
  `2026-09-10T10:15:30`, `2026-09-10T10:15:30.000+03`, `2026-253`, `2026-W37-4T10:15Z`, the
  expanded `+001985-04-12`, the comma fraction `…T10:15:30,5`;
- a form neither defines: `D:20221116191452+00'00` (refused for the `D` where a year begins),
  `2016-02-01 13:19:21` (a space), `2016-02-01T13:19:21Z+01:00` (a plus after the UTC designator),
  `2016-13-01` (a month of 13), `2016-02-01T13:19:21+0100` (basic and extended mixed),
  `…T24:00:00` (the end of a day, on a time point).

## Gates

Green, run one at a time under `tools/bounded.sh --data 12 --tree 12`: the corpus harness and
the XMP gate above; `cargo test --workspace --doc`; `cargo fmt --manifest-path fuzz/Cargo.toml
--check`; `RUSTFLAGS="-D warnings" cargo clippy --manifest-path fuzz/Cargo.toml --all-targets`;
`cargo nextest run -p pdf-archive` (203 passed, the eight new tests among them); `cargo clippy -p
pdf-archive --all-targets` silent on this crate's files.

**Four lines could not be run green, all on files sibling rounds were holding open**, none
reachable from anything this round touched:

- `cargo fmt --all --check` — diffs in `crates/pdf-render/src/shading.rs` and
  `crates/pdf-transform/src/archive/{config,mod,toml}.rs`.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets` — three errors in
  `crates/pdf-model/src/image.rs` (dead code and a constructor's field order), so the dependency
  does not compile under `-D` and no crate above it is checked; this is also why this round's own
  clippy line is `cargo clippy -p pdf-archive --all-targets` read for this crate's paths.
- `cargo nextest run --workspace` — `crates/pdf-transform/src/archive/mod.rs` names
  `report::DepartureOutcome`, `report::Departed` and `report::departure_history`, none of which
  exist yet: round 992 mid-edit. Run with `--exclude pdf-transform` under the bound: **4218 run,
  4215 passed, 3 failed, 28 skipped** — the conformance quotation test below;
  `pdf-model::transparency_groups a_knockout_group_reports_only_where_the_two_models_differ`,
  in the crate rounds 996 and 997 hold; and `pdf-vfs-ffi::the_kio_worker`, whose CMake cache was
  generated against `/home/cl/projects/pdf-viewer/kio` and refuses this worktree's path — trap
  15's shape, on a build directory rather than a binary.
- `cargo test -p conformance` — 5 passed, 1 failed: `every_quotation_is_the_standards_own_words`
  on `crates/pdf-render/src/shading.rs:200`, a §11.6.4.4 blockquote the standard does not contain.
  The crate's own test binary (234 passed) and `tests/bounded.rs`'s self-test (1 passed) are
  green; the failure is the one quotation test, on that one file.

Whoever merges owns `doc/todo/02` section 2's sequence on `main`.

## Files touched

- `crates/pdf-archive/src/iso_8601.rs` — new
- `crates/pdf-archive/src/table/metadata.rs` — `Lexical`'s header and `Date` variant,
  `refused`, the finding, `dates_stated`, the old `date` predicate removed, one test rewritten
- `crates/pdf-archive/src/lib.rs` — the module and the export
- `crates/pdf-archive/examples/dates.rs` — new
- `doc/third-party-data.md` — the working draft's section
- `doc/adr/1013-a-date-is-what-the-text-defines-and-the-text-is-a-working-draft.md` — new
- this file

`crates/pdf-model/src/xmp.rs` was read and not edited: it writes ISO 16684-1's subset of ISO
8601 and the widening is on the admitting side alone. `crates/pdf-archive/src/coverage.rs` was
read and not edited. `doc/questions/A53`'s `Owes` line — the widening, by the acting round — is
now met and is round 994's file to close.

## What the next round should know

- **The grammar is the module comment of `iso_8601.rs`, and every clause number in it is the
  working draft's.** A round that buys ISO 8601-1:2016 re-reads the module against the published
  clauses and changes the citations with the text; nothing else in the crate names the draft.
- **A NOTE of the draft defines no form here** (the omitted `T`, lower-case designators). A
  producer found writing one is a question for `doc/questions/`, not a change to the rule.
- **The census is the instrument for this row**, because the harness cannot see it: the files
  that exercise the widened forms are held to a part the row does not bind. Run
  `--example dates` after any change to `Lexical::Date`.
- **A60's part-3 price is ADR 1013 section 6**: one `Part` variant, one `Clauses` field filled
  row by row from the text, seventeen `reaches` entries in `clarification.rs`, a readings file,
  and the converter's attachment path.
