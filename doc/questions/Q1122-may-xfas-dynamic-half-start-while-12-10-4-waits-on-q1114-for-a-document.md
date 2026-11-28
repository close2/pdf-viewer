# Q1122 — May `doc/todo/78` (XFA's dynamic half) start while §12.10.4 waits on Q1114 for a document?

Asked by round 2301. This blocks nothing: until it is answered no round starts `doc/todo/78`, and the
lanes keep their present work.

## The situation

`doc/questions/A840` (2026-11-13) accepted a page laid out by a producer's own XFA template as a
near-side construction and scheduled it "after the ledger rows reach their final states", in
`doc/todo/78` section 4's order (about 23 to 28 rounds, the merge and page one's launch cost first).
`CLAUDE.md`'s fourth amendment says the same: it is built after the ISO 32000-2 ledger rows reach their
final states.

`tools/state.sh ledger` on this round's base (`4c2d361bb`): 883 rows, 802 `implemented`, 10
`departed`, 31 `inapplicable`, 2 `writer-side`, 36 `out-of-scope`, and 2 `partial`. The two partial
rows are one: §12.10.4 (projected coordinate systems), and §12.10, whose status is its subclauses'
aggregate. Annex K and §K.1 to §K.2 are `implemented` for the static half.

§12.10.4 is partial for 13 of the 5780 projected systems the carried EPSG Dataset holds, over three
projection methods — Azimuthal Equidistant (EPSG 1125), Lambert Conic Conformal (1SP variant B) (1102)
and Local Orthographic (1130) — none of which is in the September 2019 revision of IOGP Guidance Note
7-2 that `doc/` holds (`doc/todo/79`). The revision that has them (v74, July 2026) is behind a download
form that asks for a name, a company, an email address and a country, which a round may not submit
(`doc/questions/Q1114`). Until then each of the three is refused by its number and the method's name.
No round can move the row any further. The thing missing is a document, not effort.

## The options

1. **Start `doc/todo/78` now.** Treat §12.10.4 as having reached its final state *for this purpose*,
   because nothing a round can do will move it. The row stays `partial`. The three methods are built in
   one round of the geospatial lane once the document arrives, alongside the XFA work rather than ahead
   of it.
2. **Wait for Q1114.** The owner fetches v74 into `doc/` (or says what details a round may submit). One
   round builds the three methods, the row becomes `implemented`, and then `doc/todo/78` starts. The
   cost is the time until the document arrives.
3. **Close §12.10.4 another way, then start.** Record the three methods as a `departed` row: refused by
   name, with the reason that the defining text is not publicly obtainable. Then start. This turns a
   wait into a departure, and principle 5 does not ask for that: the formulas exist and are only behind
   a form.

## Recommendation

*Option 1.* A840 ordered the ledger first so that XFA's 23 to 28 rounds would not crowd out clause
work a round could still do. With every other row final, and this one held only by a document the
owner can fetch, that purpose is met. The first piece of `doc/todo/78` (the merge and page one's launch
cost, section 5) is also the one that can still find that page one costs too much, so starting it early
means that answer arrives early. Q1114 stays open on its own terms. Answering it also lets the geospatial
lane close §12.10.4 in about one round.
