# 1253 — An unread claim read against the code, and a section sign that was not the standard's

## What moved

No ledger status moved. `--bin unread` names each confirmed claim (`confirmed:`), and a solidus
inside a word (`doc/questions/Q63`, `ISO/TS`) is no longer a key. `--bin cited` lists every no-row
pair with `cited::why_no_row`'s reading of its number; `tools/state.sh unread` and `cited` print both.

Counts come from `cargo run -q -p conformance --bin unread` and `--bin cited`. Before this round:
71 rows, 159 keys, 6 confirmed, 153 quoted over 68 rows, 105 of them by the row's own code; cited
had 54 pairs with no row. After: 67, 146, 5, 141 over 64, 94; cited has 34. Siblings edit the same
ledger, so the unread delta is not all this round's.

## Findings

**Five of the six confirmed claims were already settled, and the sixth was not an entry.** `/Q63`
was `doc/questions/Q63`. `/FormType` (the only valid value is 1), `/Free` (its `shall` fires on adding
a folder), `/LineHeight` (ADR 0301) and `/MarkStyle` each had their reason written down. `/AntiAlias`
and `/MarkStyle` now quote their table sentences. All five are owed nothing, and none needed a read
built.

**Of the 105 own-code hits, 12 were stale and 17 were named without being read.** The code reads
§8.4.5's `/HT` (for a halftone's transfer function), §8.6.5.5's `/Range`, §8.9.5.1's `/Intent`,
§8.9.5.4's `/DefaultForPrinting`, §8.10.2's `/StructParents`, §12.3.5.2's `/Thumb` (only whether it
is there), §12.5.6.7's `/CP` and `/CO`, §12.6.4.3's `/SD`, §12.8.2.4's `/Data` and §14.9.2.2's
`/Lang`, and §12.7.3 said no source quotes `CO`. Each sentence now says what the code does. The
other 76 are no claim about the key: a retraction naming the old claim, a calibration of a planted
defect, past tense, a neighbouring key, or the phrase inside a word (`declared_and_unread`).

**All 20 row-less pairs that went were a `§` that meant another document**, mostly an ADR, RFC or
`doc/todo` file named at the end of the line before (`citation.rs` reads the same line only); three
were ISO 19005-4 annex sections, one `CLAUDE.md`'s principle 2. Left: Annexes B and C, clauses 2, 11.

## Handed over

- 1248: §8.9.6's `/ImageMask`, `/Mask` and `/SMask`, and §8.9.6.2's `/Interpolate`, are
  calibration or retraction sentences. They are true, and nothing was edited.
- 1249 and 1251: comment-only `§N`→`section N` edits in `viewer-confined/src/protocol.rs`,
  `viewer-gtk/src/{controls,host}.rs` and `content/transparency.rs` (two lines).
