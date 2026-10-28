# 2562 — `append-statement` states the halftone entries the graphics family takes

Status: implemented — session 1864, the fourth archive slot of batch 127; ADR 2563 and Q730 are
unused. Context: `doc/rfc/0014` sections 3 and 4, which A706 accepts whole; the shipped profile
`doc/profiles/state-every-loss.toml`; ADR 1014 section 3 (an appended page is checked against the
input); ADR 2536 (the embedded-file rows, the first place the page was set); ADRs 1965, 2034 and
2035 (the three halftone removals and their losses).

## The sites, listed by a command

`grep -n -B1 'append-statement' doc/profiles/state-every-loss.toml | grep 'site\."graphics'` names
the presentation sites of the graphics family whose list opens with the step:

- `graphics/halftone-type-is-one-or-five`
- `graphics/no-halftone-name`
- `graphics/no-halftone-origin-in-a-graphics-state`

`grep -n -i 'interpolation\|overprint' doc/profiles/state-every-loss.toml` prints nothing: the
profile takes neither `graphics/image-interpolation-is-off` nor
`graphics/no-overprint-mode-one-under-icc-cmyk`, both of which fall to its `default = "stop"`. So
the brief's "whichever of image smoothing and overprint mode it takes" is none, and the family is
the three halftone sites. Giving the profile either row is a change to what the profile authorises,
which is the profile's own decision and not this one's.

## Decision

1. **The statement's record widens.** `statement::text` lists the files an embedded-file row
   removed, as before, and then a `GraphicsEntry` per object a halftone row took an entry from.
   The page states, per entry, only facts read from the source at the time the text is set and a
   closed vocabulary fixed in `statement.rs`'s `TAKEN` table: the requirement and its citation, its
   `asks` sentence, the loss word (`halftone-name` or `halftone`) with its one line, the object
   (`halftone N G` or `graphics state N G`), the entry (`Entry removed: HalftoneName` / `HTO`, or
   `Entry written Default: HT`), its value as `pdf_syntax::write::object` writes the source's own
   value, and the pages whose resources reach the object.
2. **The objects are the rewrite's own population.** `prepare.rs`'s `stated_graphics` calls the
   same `halftones::removal`, `defaults` and `origins` the rewrite acts on, so the page lists
   exactly what the output changes; a population that refuses is the rewrite's refusal at its own
   site and nothing is stated for it.
3. **"The page using it" is a reach, bounded.** A page uses an object where the object is reachable
   from the page's resource dictionary in force (§7.8.3, inherited where the page states none) or
   from one of its annotations' appearance dictionaries (§12.5.5), through any references. The walk
   is per page under `MOST_VISITS` (2^20 values and pages); past it the loss is refused by name,
   as the statement's three other bounds are. The pages come from one walk of the tree,
   `Pages::every` (ADR 1953), and not from `Pages::get` per index: `get` reads a flat `/Kids`
   array from its start for every page, reads the budget does not count, and an 80 000-page
   document with one shared resource dictionary held the conversion past 650 s that way, where
   the walk takes 12.4 s beside HEAD's 10.9 s.
4. **A page that cannot be composed hands its objects on.** RFC 0014 section 3: "A step that fails
   for one object — … a page that cannot be composed … — hands that object to the next step". At
   the three halftone rows `append-statement` therefore hands on, so `try = ["append-statement",
   "discard"]` is a two-step list there, and where no page is composed the removal goes ahead and
   the report names no statement for the site. At the embedded-file rows a step after
   `append-statement` is still refused when the file is read.
5. **One page per document.** `StatementPage::sites` replaces `site`: the requirements whose rows
   the page states, the embedded-file row first. The report's JSON key is `sites`.

## Consequences

The profile's opening comment names the halftone rows beside the embedded-file rows.

Where an object is handed on, the report says the site was answered by `discard` and does not
yet say why the page was not composed: `statement_not_composed` in `archive/mod.rs` drops the
composition's sentence on that branch. RFC 0014 section 3 says the report names each object's
step, so the sentence is owed in the report's list, beside the step that answered. The annotations, forms and file-structure sites still pass the step over with
`STATED_ONLY_PER_OBJECT`'s sentence.
