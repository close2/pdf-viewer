# 2499 — The census of `image-smoothing` discarded and of the two-target request

Status: implemented. Session 1832, the archive slot of batch one hundred and twenty-four. It is the
measurement ADR 2498 owes. The command is `doc/pdfa-blocking-issues.md` section 1's:
`cargo test --profile gates -p pdf-transform --test archive_corpus
the_shipped_profiles_list_what_real_producers_files_refuse_on -- --ignored --nocapture --exact`, run
against HEAD's six arms' tree and against this round's, each once with a copy of the CJK pack named by
`QUORRA_CJK_PACK` (ADR 2460) and once without, and read with
`python3 -I -B doc/pdfa-blocking-issues/rank.py <log> --repository . --render <quorra-transform>`.

## R02: documents written, before and after (failing plus conforming)

| profile and part | with the pack, pdf.js | with the pack, corpora | without the pack | freed |
|---|---|---|---|---|
| `as-if-printed` 2b | 806 to 829 | 357 to 373 | same freed | 23 + 16 = 39 |
| `as-if-printed` 4 | 807 to 830 | 353 to 366 | same freed | 23 + 13 = 36 |
| `as-shown` 2b | 770 to 793 | 310 to 323 | same freed | 23 + 13 = 36 |
| `as-shown` 4 | 772 to 795 | 304 to 315 | same freed | 23 + 11 = 34 |

The without-pack runs move by the same 39, 36, 36 and 34; the pack adds eleven pdf.js documents under
`as-if-printed` at each part (795 to 806 at 2b before the change, 818 to 829 after) that no R02 touches.

ADR 2490's figures were upper bounds, "R02 41/39": the ranking's `alone` count at `as-if-printed` 2b and 4
was 41 and 39 and 38 and 37 under `as-shown`. The round freed 39, 36, 36 and 34, so two to three documents
per census line were refused by something that stood behind the smoothing: `Wiki SI 563 2021.pdf` and
`CLLASS Report for ER&L 2020.pdf` by `annotations/flags-entry-present` (every line), and at part 4
`Montenegro2021.pdf`, whose one signature is a loss nobody authorised, a refusal that is no decision of
the table's and shows in the census as `-`. Of the 43
documents behind R02, 4, 7, 7 and 9 are still refused under the four lines, each by another site named in
the log (the font sites, `device-cmyk-needs-a-default...`, the same two annotation sites).

## R06b: what `--to 2b --or-else-to 4` writes

The census gains a line per corpus and profile: the documents written at 2b, those written at 4 only
because 2b refused them, and those both refuse.

| profile | corpus | written at 2b | written at 4 after 2b refused | refused at both |
|---|---|---|---|---|
| `as-if-printed` | pdf.js | 829 | 17 | 109 |
| `as-if-printed` | corpora | 373 | 9 | 91 |
| `as-shown` | pdf.js | 793 | 17 | 145 |
| `as-shown` | corpora | 323 | 4 | 147 |

So the request writes 26 more documents under `as-if-printed` and 21 under `as-shown` than `--to 2b`
alone; ADR 2490's upper bound was 26 (the ranking's greedy figure at `as-if-printed` 2b). Without the pack the
`as-if-printed` pdf.js line reads 818 written at 2b and 120 refused at both; the others are unchanged.

## The planted and the freed, on veraPDF 1.30.3

Every document the R02 ranking names (43, in the union of the four lines) was converted at both parts
under both profiles and every written file validated at the part it was written for: 145 files written
(the rest are the 27 conversions the other sites still refuse), 143 passed and 2 failed (`475121.pdf` at 2b under each profile, 40 content-stream integers past 2147483647, ISO 19005-2
section 6.1.13, which this tree's validator does not read in content streams; the same file converted by
HEAD with `--authorise image-smoothing` fails alike, so it is the validator's gap and not this round's
rewrite, and part 4 lifts the limit and passes). The 47 documents of the R06b rows were converted with
`--to 2b --or-else-to 4` and validated at the part their `pdfaid:part` states: 45 passed and 2 are
`poppler-67295-0.pdf` under each profile, on which veraPDF throws a runtime exception; the same file
converted by HEAD with `--to 4` is byte for byte the file the request writes, so the construction is not
new (trap 267's class: a refusal that stood in front of a validator gap).

Both remedies were planted away once: with the R02 entry set back to `stop` two tests fail (the profile
authorises the loss; a smoothed image converts under each profile), with the fallback disabled two tests
fail (the second target writing the file; both refusing).
