# 1881 — a page the file does not hold is the named loss `page-not-held`

Session 1881, batch 129, slot 1 (A727), in two sessions. ADR 2596; Q757 to the owner; ADR 2597 unused.

**What changed.** `Loss::PageNotHeld` (`decision.rs`, the word `page-not-held`; `Losses` is 64 bits, since 33 losses
no longer fit in 32). The rebuild (`recovered.rs`) skips an index the reader cannot read and answers the source's
stated count — the root's `/Count` or `/N` — against the pages carried. `page_tree::children_not_held` counts, under
the tree walk's bounds, the `/Kids` entries naming no object (§7.3.10's null: a direct non-dictionary or a reference
placed nowhere), which are no page and cost only `recovered-structure`, and those placed with no dictionary there; a
shortfall past the first is a page not held, refused with "authorise page-not-held" unless taken, and when taken
the two counts go into the report, its JSON (`page_not_held`, `kids_naming_nothing`) and `xmpMM:History`.
`[profile] page-not-held` reads `discard` or `append-statement` (`config.rs`; `Authorisations::page_not_held_stated`,
copied by the CLI); `as-if-printed` takes `discard`. `statement.rs`'s `not_held_entry` lists the loss, both counts and
the clause stating the count on ADR 1014's page; the page's intent is prepared as for ADR 2539's pages, and a page
that cannot be composed refuses the source with its reason rather than taking the loss unstated (`mod.rs`). Lines in
`doc/pdf-a-mitigations.md`, `doc/pdfa-blocking-issues.md` (R01) and `doc/pdfa-remedies.md` (a `page-not-held` row).

**The premise that did not hold.** Both shipped profiles already took `recovered-structure`; the five documents were
refused by the rebuild's own "page N of the M this reader counts cannot be read". Read against the file the split is
§7.3.10's: `T02-02_003`, `10856087…` and `66307685…` hold a kid naming nothing; the Defence document and
`14342427…` name objects the table places inside other data. `T02-03_006` holds no page and stays refused. A727's
statement page under `state-every-loss` and `no-data-loss` cannot be reached, since neither takes
`recovered-structure` (Q757); nothing is built for them.

**Measured.** Census against HEAD's, no-pack and pack alike: `as-if-printed` converted 406 → 411 at 2b and 411 → 416
at 4, `as-shown` 338 → 341 and 336 → 339; every moved line is one of the six documents (`T02-03_006` now "draws no
page", `as-shown`'s Defence and `14342427…` name page-not-held); `rank.py` exit 0 on all four logs. veraPDF: 10 of
session one's 11 outputs compliant. Under `page-not-held = "append-statement"` `14342427…` converts with the entry on
page 54 (part 4 compliant) and the Defence document is refused by name: its page labels are a number tree with
`/Kids`, which the appended page does not extend. A planted `/GTS_PDFX1` intent failed 6.2.4.3 at both targets.

**Found, not fixed.** `14342427…` at 2b fails ISO 19005-2 6.6.2.3.1 on `xmpMM:Ingredients` in object 553's packet,
under both profiles' outputs, and `pdf-archive`'s metadata rows pass it: a checker gap.

**Gates.** `cargo test -p pdf-transform --test archive --test profiles` exit 0 (512 and 13 passed; `recovered::` 22,
the plant reading 13 and 12 back from the report, JSON, history and statement page, a profile without the loss
refused by name). Clippy `--no-deps -D warnings -p pdf-transform --all-targets` exit 101 (two functions past 100
lines, three fixture sums, this round's), exit 0 after the fix. `--bin cited` exit 0; `--bin quotations` exit 0
(2851 of 3049 verbatim). `cargo test -p conformance --no-fail-fast` exit 101: `records` (1885's record, since green)
and `evidence` (1887's record), siblings' in progress. Census two runs exit 0; veraPDF 6 runs this session, 3 failed.
