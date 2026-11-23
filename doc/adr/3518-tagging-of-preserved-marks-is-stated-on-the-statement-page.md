# 3518 — `tagging-of-preserved-marks` is stated on the statement page

Session 2204. Status: **accepted**; built. Context: ADR 3509 decision 6, A706 and A707, ADRs 2596,
2669 (the same key shape for `page-not-held` and `recovered-structure`); `CLAUDE.md`'s first authoring
amendment; ISO 32000-2 §14.8.2.2.2.

## Decision

1. **`[profile] tagging-of-preserved-marks = "append-statement"`** takes the loss and states it, as
   the two structure losses' keys do; `discard` takes it unstated, anything else refuses.
   `Authorisations::tagging_of_preserved_marks_stated` carries the second half; it is no loss of its
   own, and `--authorise tagging-of-preserved-marks` alone states nothing.
2. **One entry an annotation**, `statement::marks_entries`, in the statement's closed vocabulary:
   the annotation row's identifier, citation and `asks` sentence from the table; `Loss:
   tagging-of-preserved-marks` with one fixed line (`MARKS_LOSS`); and three facts the removal
   recorded — the annotation's object, its `/Subtype` as the file spells it (escaped as every name
   on the page is), and the producer's page it was on. These are the facts the report's row subject
   already names it by, so the page and the report name one annotation alike. The appended page the
   marks went to is not on the statement, because the statement is composed before the pages are
   counted; the report and `xmpMM:History` name it.
3. **`state-every-loss` states it**; `as-shown` keeps `discard`, since it states nothing anywhere.
   The statement's report sentence counts the entries as "annotation(s) outside the structure
   tree" (`Listed::marks_records`).
4. **Within the permission.** Nothing on the page is invented: every value is the file's or this
   program's fixed words, and the page carries no other text (A706).

## Proof

`tests/archive/tagging_of_preserved_marks.rs`: at 2b and 4 the statement follows the marks' page,
names the row, the loss, `Sound annotation 6 0` and `Page it was on: 1`, and the report counts one
entry; `state-every-loss` states the loss and `as-shown` does not.
