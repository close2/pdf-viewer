# 2669 — `state-every-loss` and `no-data-loss` take a recovered structure and a page not held, each stated on the appended page

Session 1921, a single archive round.
Status: accepted and built in `doc/profiles/state-every-loss.toml` and `no-data-loss.toml` and in
`crates/pdf-transform/src/archive/` (`config.rs`, `decision.rs`, `recovered.rs`, `statement.rs`,
`prepare.rs`, `prepare/appended_pages.rs`, `mod.rs`).
Context: `doc/questions/A757` (Reading 1, bounded by each profile's contract), ADR 2502 (the rebuild
and the loss `recovered-structure`), ADR 2596 (the loss `page-not-held` and its statement entry),
ADR 2562 (the statement page's closed vocabulary), ADR 1014 (the appended page), ADR 2670 (the same
round's A775, which refuses a document holding none of its pages before any of this).

## The question

`page-not-held` is asked only of a source the archive rebuilds, and a source is rebuilt only under
`recovered-structure`. Of the shipped profiles only `as-if-printed` and `as-shown` took that, so A727's
loss could not reach `state-every-loss` or `no-data-loss`: a document such as `A Defence of the New
Tenseless Theory of Time.pdf` was refused by both for its structure before its count was read. The
owner adopted Q757's recommendation: both profiles take both losses, each stated.

## Each profile's own contract

- **`state-every-loss`** says "Convert whatever the window can show: keep what can be kept, and state
  every loss taken on an appended page." The window shows a declined source's pages, so the profile
  converts it; the structure is the loss taken, and it is stated. A page the tree counts and the file
  does not hold is a second loss, stated beside it.
- **`no-data-loss`** says "No information is lost: it is kept, moved or written out on an appended
  page; a presentation loss is discarded and stated." What `recovered-structure` changes is how a
  reader reaches the pages — the catalog and the tree's membership and order — while every page
  dictionary, content stream and resource is carried as the producer wrote it (ADR 2502 decision 2),
  so it is a presentation loss (`doc/pdfa-remedies.md`'s class column) and is discarded and stated.
  A page not held is data the file names and does not hold: nothing of it is in the source to keep,
  so the profile takes the loss only as `append-statement`, where the count the file names is written
  out on the appended page. It never takes `discard` for either.

## Decision

1. **`recovered-structure` gains `append-statement`**, read as `page-not-held`'s already is: `discard`
   takes the loss, `append-statement` takes it and states it, and anything else keeps the refusal.
   `Configuration` holds each as one `StructureLoss` (refused, discarded, stated), and
   `Authorisations::recovered_structure_stated` carries the wish beside `page_not_held_stated`; neither
   is a loss of its own, and `grants` reads the loss alone.
2. **The entry.** One numbered item in ADR 2562's closed vocabulary: the loss word and one fixed line,
   then facts the rebuild read from the source — the count it states and where (the root's `/Count`,
   Table 30, or a linearization dictionary's `/N`, Table F.1), or "no count is stated"; the pages
   this document holds, each the producer's; how many were found by their own `/Type /Page`
   declaration; whether the catalog is the producer's or written; how many `/Kids` entries named no
   object; each page read in part. It names no ISO 19005 requirement, because none declines the
   source. `RecoveredStructure::stated` carries the stated count whether or not every page is held,
   so the entry sets the root's declared count against the count carried in every case.
3. **Order and bounds.** The structure's entry precedes `page-not-held`'s, both after every row's
   entries; the record lists them under `recovered-structure` and `page-not-held`. The entry is held
   to `MOST_STATEMENT_BYTES`, and its pages read in part to `MOST_STATED_OBJECTS`.
4. **Taken only with its page**, as ADR 2596 holds `page-not-held`: where the appended page cannot be
   composed, the source is refused with the composition's reason and the losses it was to state.

## What moves

The census is `crates/pdf-transform/tests/archive_corpus.rs`'s
`the_stating_profiles_list_what_real_producers_files_refuse_on`, the same sweep as the lower-bound
profiles' census over `doc/pdf.js` and `doc/corpora` at 2b and part 4, run once with the profiles
before this change and once after:

    tools/bounded.sh --lock --tree 6 --build '--profile gates -p pdf-sandbox -p pdf-transform --bins' -- \
      cargo test --profile gates -p pdf-transform --test archive_corpus \
      the_stating_profiles_list_what_real_producers_files_refuse_on -- --ignored --nocapture --exact

No document is refused that was not, and none that converted before changes output: the rebuild runs
only for a source the structure checks decline, which both profiles refused. What converts now and
did not, as documents (corpora + pdf.js):

| profile | 2b | part 4 |
|---|---|---|
| `state-every-loss` | 15 + 3 | 16 + 3 |
| `no-data-loss` | 15 + 1 | 16 + 1 |

They are `108560872853…`, `663076857724…`, `PDFBOX-6040-nodeloop`, `T02-01_001` to `_005`,
`T02-02_001`, `_003`, `_004`, `_005`, `T04_012` to `_014`, `issue9418`, at both targets;
`143424271191…` at part 4 only; and `issue19484_1` and `_2` under `state-every-loss` only. Every
one of the 70 files those conversions write passed veraPDF 1.30.3 at its target.

## What stays refused, and the elegant remedy

- **The appended page cannot be composed**, so the losses are not taken unstated: the Defence
  document (its page labels are a number tree with `/Kids`, which ADR 1014's page does not extend),
  `143424271191…` at 2b (the pages the statement would append pass the conversion's page budget),
  `issue9105_other` (its first page states no usable `/MediaBox` for the appended page to take) and
  `poppler-742-0-fuzzed` (its XMP packet cannot record the appended page in `xmpMM:History`, ADR 1014
  section 5). The remedy for the first is the appended page extending a number tree's labels; none
  is built here.
- **`issue19484_1` and `_2` under `no-data-loss`**: `graphics/only-operators-the-base-standard-defines`
  refuses them, a row `no-data-loss` answers with ADR 0816's fence; the structure no longer does.
