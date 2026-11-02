# 2596 — A page the tree counts and the file does not hold is the named loss `page-not-held`

Session 1881, slot 1 of batch 129, an "archive" round.
Status: accepted and built in `crates/pdf-transform/src/archive/` (`recovered.rs`, `page_tree.rs`,
`config.rs`, `decision.rs`, `statement.rs`, `prepare.rs`) and `doc/profiles/as-if-printed.toml`.
Context: `doc/questions/A727` (both items of its `Owes:` line), ADR 2502 (the rebuild and its
refusal of a count the reader fell short of), ADR 2558 (the census that found the six), ADR 2562
(the statement page's closed vocabulary), ADR 1014 (the appended page). Q757 asks what A727 left
open for `state-every-loss` and `no-data-loss`. ADR 2597 is unused.

## 1. Two kinds of kid, and the clause that splits them

ISO 32000-2 §7.7.3.2 Table 30 makes `/Kids` an array of indirect references whose children "shall
only be page objects or other page tree nodes", and its NOTE under `/Count` calls the count redundant
beside the tree it summarises. §7.3.10 says an indirect reference to an undefined object is no error
and is treated as a reference to the null object. So a `/Kids` entry that is a direct non-dictionary,
or a reference the cross-reference information gives no location, **names no page**: it is §7.3.10's
null, dropping it changes nothing a reader shows, and the rebuild drops it under the loss
`recovered-structure` that both `as-if-printed` and `as-shown` already take (A727 item 2). The count
check subtracts such kids from the root's `/Count` before it compares (`page_tree::children_not_held`,
a bounded census of the entries that are no dictionary).

A reference the cross-reference information **does** place, where no dictionary is found, is a page
the producer wrote and the file does not hold. Nothing the standard says turns it into null, so it
is not dropped silently: it is the named loss `page-not-held` (A727 item 1). Without the loss the
rebuild refuses with "authorise page-not-held"; with it, the pages carried are written and
`recovered::NotHeld { stated_by, stated, carried }` — the count the root's `/Count` (or a
linearization parameter dictionary's `/N`, Annex F.3.3 Table F.1) states and the count carried —
goes into the report's sentence, the JSON (`page_not_held`, `kids_naming_nothing`) and the file's
own `xmpMM:History`. No page is invented for the one not held; that is the far side of the
authoring line.

## 2. Why the census split differs from Q727's

Q727 read the five non-Defence documents as refused for lack of `recovered-structure`. They were
not: both shipped profiles already take it, and the five were refused by the rebuild's own "page N
cannot be read" at an index the reader could not read. The rebuild now skips such an index (it is
either a kid naming nothing or a page not held, and the count check decides which), so item 2 is
the count check's Table 30 subtraction and no new profile line.

## 3. The statement page, and the profiles

`[profile] page-not-held` reads `discard` (take the loss) or `append-statement` (take it and list it
on ADR 1014's appended page); anything else keeps the refusal. The page's entry is one numbered
item in ADR 2562's closed vocabulary — the loss word, one fixed line, the two counts and the clause
that states the count — and names no ISO 19005 requirement, because none declines the source; the
statement page's record lists it under the loss word. `Authorisations::page_not_held_stated` carries
the wish; `grants` reads `page_not_held` alone, and the CLI copies it beside the losses it folds in. The page paints in Table 51's initial `DeviceGray`, so an output intent is prepared for it as for ADR 2539's appended pages. **A loss asked stated is taken only with its page**: where ADR 1014's page cannot be composed (the Defence document's page labels are a number tree with `/Kids`, which the appended page does not extend) the source is refused with the composition's reason, never taken and stated in the report alone. `as-if-printed` takes `discard`: a print of the file
shows the pages it holds, and its shape states no loss on a page. `as-shown` keeps the refusal (A727
does not name it). `state-every-loss` and `no-data-loss` take no `recovered-structure`, so the loss
cannot reach them; Q757 asks whether it should, and nothing is built for them.

## 4. What it costs

`Loss` has 33 variants with this one, one past a `u32`'s bits, so `Losses` is a `u64`. The census of `/Kids` entries is
bounded by the page tree's own bounds, before the work. Measured: `as-if-printed` converts the
Defence document, `T02-02_003`, `10856087…`, `14342427…` and `66307685…`; `as-shown` converts three
and refuses the Defence document and `14342427…` by name (`page-not-held`). `T02-03_006` holds no
page and stays refused ("draws no page"). veraPDF passed ten of the eleven outputs; the eleventh
fails ISO 19005-2 6.6.2.3.1 on `xmpMM:Ingredients` in a packet this checker passes — a gap in our
checker, named in the record and not fixed here. With `page-not-held = "append-statement"` over `as-if-printed`, `14342427…` converts at both targets with the entry on page 54 (veraPDF: part 4 compliant, part 2 the same 6.6.2.3.1 gap), and the Defence document is refused by name for its page labels.
