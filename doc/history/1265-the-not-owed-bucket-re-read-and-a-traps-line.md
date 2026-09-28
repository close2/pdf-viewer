# 1265 — The not-owed bucket re-read, and a `traps` line

Instruments slot, batch forty. ADRs 1367 and 1368; `doc/questions/Q168`.

## Rows (each read against its clause, the titles either side, and `spec-errata emit`)

- §12.6.4.9, §12.6.4.10: `reported` → `out-of-scope`, `clause-13-multimedia`. Each clause opens by
  handing itself to 13.2, the position §12.6.4.14 is in. Emit files nothing under either heading.
- §12.5.6.11, §12.5.6.12: `reported` → `departed`. A72's bound decided the artwork, so it is not
  debt. ADR 1367 prices it: 6 of 6 carets and 19 of 19 stamps in the corpus carry an `/AP`.
- §12.6.4.6: stays `reported`, and the not-owed reading failed. Table 207's `/F` is "the document
  that shall be opened" as well as an application. The corpus's one launch action names
  `file1.pdf` with `/NewWindow true`, and it is told `running an application`. The fix is the
  existing remote-documents act (ADR 1368, bucket 6). It is unbuilt, because the viewer crates are
  1260's.
- §7.6.5–§7.6.5.3: no note named A66's trigger, and now each does. `doc/corpora/pdfbox` holds four
  public-key documents and the two keystores whose issuer and serial match their recipients
  (checked with `openssl`). So Q66's claim that none of the five could be decrypted is false for
  four. They are a witness and not a user's certificate: Q168, recommending *not a trigger*.
- §12.7.5.4 is `implemented` and was already out of the bucket.

## Indexes

`doc/todo/65`: the not-owed bucket is empty, bucket 6 holds §12.6.4.6, and bucket 5 names the
witness. `doc/todo/README.md`: "closed by decision" no longer lists submit or GoToR (both built) or
launch whole. Line 26 says nothing is owed. Lines 11 and 64 stop restating their files' lists. The
index matches `ls`. Todo numbers 17, 18, 20, 24 and 54 are cited only from records and fixtures.
`doc/verify.md` gains `ccitt_decoder_census`. `doc/crate-map.md` names the census in `pdf-ccitt`'s
row and `redact.rs` in `pdf-transform`'s. `doc/HANDOVER.md` gains a row for fetching a free text.
`doc/third-party-data.md` now says the T.4/T.6 PDFs are kept at `/home/AI/specs/` (hashes match).
In the four navigational documents, nothing batch thirty-nine touched is false.

## `tools/state.sh traps`

It counts the index rows, the group files' `wc -l` and trap citations across `doc/history/`: a top
ten, how many traps carry 80%, and cited numbers with no row. It is in `quick`.

**Left**: the §12.6.4.6 build (ADR 1368), and Q168.
