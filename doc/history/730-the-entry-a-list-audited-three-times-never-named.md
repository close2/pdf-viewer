# 730 — The entry a list audited three times never named

The ledger's `partial` rows read as a family for the sixth round running, on ADR 0538's method for
the ninth block, with the pair chosen by ADR 0567's search under 0593's third rule — **take the
strongest pair the previous round named and did not read.** 725 named three pairs and every one of
them is now spent, so the rule ran out above the fourth rank and the fourth rank is a tie; 0579's
rule broke it, and the pair is **§8.6.5.8 ~ §8.9.5.1**, a family no round of this method has opened.

Date: 2026-08-25.
ADRs: [0620](../adr/0620-the-entry-a-list-audited-three-times-never-named.md),
[0621](../adr/0621-an-erratum-read-one-page-at-a-time.md).

Touched: `doc/conformance/ledger.toml` (§8.6.5.8's note, §8.9.5.1's note and its `code` array),
`crates/pdf-model/src/content/colour.rs` (two doc comments on `Intent`), `doc/errata-read.md` (one
paragraph and two table rows), `doc/todo/01`, the two ADRs and this file. **No status moves, no
pixel moves, and no report is added or removed.**

## Why the pair, and the ranking run

The search was run rather than read out of a document, with 710's two rules and 716's third. **The
head did not move** — §12.5 first, §12.8 second — and the three strongest pairs below any
clause-level parent are the same three ADR 0610 §1 names: §12.4.4 ~ §12.4.4.1, §12.8 ~ §12.8.3 and
§10.7.4 ~ §10.7.5. 0600 read the first, 0567's round wrote §12.8.3 and its whole subtree, and 0610
read the third. So the third rule has nothing left above rank 4, and rank 4 is a **tie at 31 shared
rare sequences** between §11.4.7 ~ §11.7.2 and §8.6.5.8 ~ §8.9.5.1.

0579's rule chose: prefer the pair whose two rows do not merely quote the same sentence but
*disagree about what it leaves standing*. The §11 pair's shared text is a long narration about the
press budget and they agree throughout. The §8 pair's rows disagree outright — §8.6.5.8's opening
list says an image dictionary's `/Intent` is unread, its own later paragraph closes it, and
§8.9.5.1 cites the opening sentence as current.

## The findings

- **The fourth audit's hole was a fifth: Table 87 states `/OC` and §8.9.5.1's row disposes of it
  nowhere.** The row is a disposition of Table 87 and its own prose says a list wrong three times
  about itself is a list to check; the five-hundred-and-twenty-fifth checked the unread five and the
  five-hundred-and-eighty-second found four entries disposed of neither way. There were five. `/OC`
  appears once in the note, as an `/Alternates` member's own key under §8.9.5.4 — Table 89's, not
  this table's — and the entry left out is the one carrying two `shall`s that decide whether the
  image is drawn at all. Both are executed, in `content/xobject.rs`, which is neither of the files
  the row lists. **Three sweeps sit over it and none can print it**: `--bin entries` wants an entry
  the row's code does not name and six files here name `/OC`; `--bin unread` *does* print it, on the
  rung it marks as noise, because the alternates' `/OC` put the word in the note; and the row's own
  opening list gives `/Filter` and `/DecodeParms` to Table 87 where this clause's first sentence
  puts them in Table 5, which `--bin tables` cannot see because a parenthesis three keys in ends its
  reach.
- **`A2B1` named as a table this tree does not select, in the row that also says it reads it.**
  §8.6.5.8 said twice that selecting a profile's `A2B1` or `A2B2` by intent is not done, and once
  that `A2B1` "is that table, and it is what this tree reads". `icc.rs` settles it — `A2B1` first,
  `A2B0` as the fallback, `A2B2` never — and the row's own `6696954.pdf` paragraph calls `A2B0` the
  perceptual table three sentences earlier. So the pair a `Perceptual` or a `Saturation` would need
  is the profile's *other* transforms, and **the module that owns the choice had it right the whole
  time**: `icc.rs` says rendering intents beyond picking `A2B1` over `A2B0` are not modelled. 710's
  shape, and it reached the code too — `content::colour::Intent`'s doc comment carried the same
  wrong pair, and beside it cited **Table 52** for an initial value **Table 51** states, the
  device-dependent list for a device-independent parameter. The ledger row had that number right.
- **An erratum read one page at a time.** `emit` over the two clauses found Issue #63 on §8.6.5.8
  and #13, #14, #366, #215 and #619 on §8.9.5.1's pages, and the first two are already in
  `doc/errata-read.md`. **Issue #619 is not**: that file records it as adding a deprecation notice
  to Table 143's `/ID` and `/OPI`, and closes "Table 143 states it for both entries now and Table 87
  still states it for one". The issue carries **four** carets. One is on page 275 in Table 87's own
  `/ID` row and one on page 121 in Table 31's, so it repairs exactly the unevenness that paragraph
  diagnosed. Verified with that section's own arithmetic: page 275's rect is 658.2–665.6 from the
  top of an 841.92-tall page, on the `/ID` value cell at 654.9, its centre at x 395.2 one glyph
  inside `preferred)` which ends at 399.2. Page 121's row also carries Issue #106, which strikes
  "; indirect reference preferred" from the same parenthetical — the two **compose** rather than
  collide, which is the ordinary case beside ADR 0601's pair.

**The shape of the third finding is 725's**, and that is why it is written down: the paragraph it
corrects was not stale. It was wrong when written, from facts that were all right, because the
reasoning stopped at the page in front of it — and `emit` files by page, so an issue whose carets
are scattered across three tables prints under three clause headings hundreds of lines apart.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this is a fifth round and the change touches `pdf-model`, so §2 ran whole and
§5's binaries were rebuilt and installed — `round.sh` had flagged `target/` as holding none of them.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green. **The whole sequence ran twice**,
because putting `A2B2` back into the doc comment and rewriting one pointer came after the first
pass; the second pass is the one that counts and `cargo test -p conformance` was the last thing run.
**The three lines that spawn another program were held until the load fell**, which is §2's own
rule: they were first offered a one-minute load average above 60 on 24 cores and were run at 11 and
below instead. The second pass's oracle line ran while a neighbour picked up again, at 52, and
reported the same verdicts and the same contradicted pages in 41 seconds against the first pass's
29; the extraction cache took 958 hits and no misses on both. The only clippy output either time was
`viewer-qt`'s cold-build gcc `-Wmaybe-uninitialized` lines, which §2 documents as not lints.

Thirteen sweeps run before the edits and after them, with the four errata commands beside them.
`quoted` and `unpriced` were not run: this round touches no page-list note and both take the
oracle's log as their right-hand side.

**One level moved into a defect bucket on this round's own prose and it was put back, and the
mechanism is worth the words**: `--bin owed` went 179 unnamed terms over 112 rows to **181 over
113**, and §8.6.5.8 fell off the reading list. Two causes, both this round's. The first is real and
is the finding — `A2B2` was named by exactly one source, `content::colour::Intent`'s doc comment,
which named it only to say the tree does not select it, so correcting that comment left the ledger's
`A2B2` witnessed by nothing. The comment names `A2B0` and `A2B2` outright now, which is what the
reading established, and the term is witnessed again. The second was noise of the ordinary kind: the
new note wrote the pointer `content/xobject.rs`, and a solidus followed by letters is a `/Key` to
the sweep's own tokeniser, so `xobject` became a term no source names. It is written
`xobject.rs::draw_xobject` now — a symbol pointer `--bin pointers` resolves — and the level is back
at 179 over 112.

**One level improved and it is a second witness for the finding.** `--bin entries` went 179 reported
entries to **177**: §8.9.5.1's `code` array gains `crates/pdf-model/src/content/xobject.rs`, which
is the file that reads Table 87's `/OC` and `/Subtype`, and the sweep stops counting those two as
entries the row's own code does not name. That the row's code list did not hold that file is part of
what this round found.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Final
levels, after ← before: `counts` 7806 ← 7774 sentences with 411 attributed counts both times,
**58 "no such way" and 4 places counting one family twice unchanged**; `quotations` 6164 ← 6141
document quotations over 932 ← 929 documents with **diverging unchanged at 37**, and 1925 ← 1922
ledger quotations with **diverging unchanged at 2**; `tables` 6466 ← 6438 sentences and 2425 ← 2414
key citations with **absent unchanged at 100, contradicted denials at 6 and keyless at 58**;
`pointers` 8099 ← 8089 with **absent unchanged at 131 and undefined at 13**, and 131 ← 130 symbol
pointers, the new one resolving; `owed` 3816 ← 3813 terms with the two figures above; `overtaken`
555 ← 553 decision records with **41 overtaken unchanged**; `blockers`, `unread`, `inapplicable`,
`overstated`, `capabilities` and `callers` byte-identical. `spec-errata moved` is byte-identical,
`check` differs only in the line numbers this round's insertions into `doc/errata-read.md` shifted,
and `applied` grows by one quotation of retired text — ADR 0621 quoting the words Issue #106 struck
— which the sweep files under *a correction quoting the wording it retired*, leaving **10 carrying
no mark of a correction, unchanged**, which is that sweep's own read-first list.
