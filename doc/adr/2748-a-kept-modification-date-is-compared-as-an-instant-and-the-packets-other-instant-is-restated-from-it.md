# 2748 — A kept `/ModDate` is compared as an instant, and the packet's other instant is restated from it

Session 1968, a single archive round.
Status: accepted and built in `pdf-transform` (`archive/prepare/metadata.rs`: `names_the_same_instant`,
`xmp_date`, `information_date`, `RESTATED`, `Information::restated_modification` and
`restated_history`, the cut in `prepare_metadata`; `archive/prepare.rs`: `NO_PLACE_TO_RECORD_A_DATE`;
`archive/config.rs`: `disagreeing`), and three profiles. It **revisits** ADR 1269 section 4's last
paragraph and builds R18 of `doc/pdfa-blocking-issues.md`.
Context: ADRs 1269, 2358, 2490, 2693; `doc/questions/A48`.
Clauses: ISO 32000-2 §7.9.4, §14.3.3 (Table 349), §14.3.4, §14.5 (Tables 350, 351); ISO 19005-4
sections 6.1.3 and 6.7.5.

## 1. The premise, checked

R18's three documents are refused at part 4 by one sentence, MODIFICATION_DATES_DISAGREE: the catalog
states a `/PieceInfo`, so ISO 19005-4 section 6.1.3 keeps `/Info` holding `/ModDate` alone, and the
packet's `xmp:ModifyDate` was not the same text. It held for two of three:

- `225188.pdf`: `D:20060922094100-07'00'` beside `2006-09-22T09:41-07:00` — **one instant**, the packet
  stopping at the minutes. The comparison was of spellings.
- `20789_ftp.pdf`: `D:20080515152007+05'30'` beside `2008-05-13T12:35:28-04:00`; the data dictionary's
  `LastModified` is `D:20080513123528`. Something rewrote the dictionary two days later and left the
  packet.
- `UMTRI-2008-80.pdf`: `D:20230227192022Z00'00'` (macOS Quartz, appending in 2023) beside
  `2008-09-02T11:04:34-04:00`.

Under `as-if-printed` at 4 it is each one's only refusal; under `as-shown` each also stops at a font
loss (R09). At 2b none is refused: ISO 19005-2 states no such rule.

## 2. Equivalence is of instants

§14.3.4 asks that the two sources be "fully equivalent" where both are written. A date in either grammar
names an instant, and the comparison is now of that: the XMP Date (the W3C forms ISO 16684-1
references) is read into §7.9.4's fields, a field it leaves out read with §7.9.4's own default (month and
day 1, the rest zero), an offset moving the clock and not the instant. Two refinements keep it from
deciding what a producer meant: a zone stated on one side only is not equivalence (§7.9.4 reads an
absent offset as GMT for comparison, but an absent zone states nothing), and a fraction that is not zero
names no second §7.9.4 can write. `225188.pdf` converts under every profile and nothing in it changes.

## 3. Where the instants differ, §14.5 says which one stays

ADR 1269 refused here because choosing between the producer's two dates was deciding which statement
was true (A48). There is a reason to keep one that is not a judgement of truth. §14.5 compares a data
dictionary's `LastModified` with "the ModDate entry of the document information dictionary", for
equality only, "to ascertain whether the data dictionary corresponds to the current content"; nothing
compares it with the packet. Table 349 makes the entry required beside a `/PieceInfo` for that reason.
Changing the dictionary's value would change the answer §14.5 gives about the private data — in
`20789_ftp.pdf` it would make a stale data dictionary read as current. So the entry stays byte for byte,
and the packet's property is the one restated: removed, and written again as the dictionary's instant,
spelled as every date this converter moves into a packet is spelled.

That is a loss — the packet's former value — and it is named: the output's `xmpMM:History` records
`xmp:ModifyDate <former> restated as the document information dictionary's ModDate … (modification-date)`,
the report's `moved_information` names the answer, and a packet that will not take the entry withdraws
the restatement (`NO_PLACE_TO_RECORD_A_DATE`, ADR 2693's rule).

The alternative not taken: write the conversion's own instant into both, since the conversion is the
most recent modification. It loses both producer dates and would change every `/PieceInfo` output, also
those whose dates agree; it is a decision about every output's dates, not this site's.

## 4. Per profile

A new row key, `disagreeing`, at the two section 6.1.3 sites: `discard` restates, and every other word
or none is `stop`.

- **`as-shown`**: `discard`. A modification date is drawn on no page.
- **`as-if-printed`**: `discard`. Paper carries no packet.
- **`only-metadata-loss`**: `discard`. Its sentence is this loss.
- **`no-data-loss`**, **`keep-everything`**, **`refuse-any-loss`**: `stop`. A value goes.
- **`state-every-loss`**: `stop`, because its rule is that a loss is stated on the appended page, and
  that page's information list states a removed `/Info` entry, not a packet property. Stating this one
  there is its own construction.

## What it costs

`as-if-printed` at 4 converts all three; `state-every-loss` at 4 converts `225188.pdf`. No page is
touched.
