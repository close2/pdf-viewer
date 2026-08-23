# 677 — The entry a halftone carries that is not a halftone

§10.5's ledger row named its own remaining debt: a halftone dictionary's `TransferFunction` "shall
override the corresponding one specified by the current transfer function parameter in the graphics
state", and Table 57's `/HT` was read by nobody here on the grounds that §10.6's halftones are
inapplicable. **This round took that seriously rather than assuming it**, read §10.5 whole, read
Table 57's `/HT` and read §10.6's own condition for inapplicability — and found that the condition
is about a *screen* and the entry beside it is not one. So the override is owed, and it is
implemented.

Date: 2026-08-23.
ADR: [0505](../adr/0505-the-entry-a-halftone-carries-that-is-not-a-halftone.md).

Touched: `crates/pdf-model/src/content.rs`, `src/content/ext_gstate.rs`, `src/content/image.rs`,
`src/content/pattern.rs`, `src/content/transparency.rs`,
`crates/pdf-model/tests/transfer_functions.rs`,
`crates/pdf-model/examples/transfer_function_census.rs`,
`doc/conformance/ledger.toml` (§10.5, §10.6, §10.6.5, §10.6.5.1 to §10.6.5.6, §11.7.5.2),
`doc/errata-read.md`, `doc/todo/13`, the ADR and this file.

## The answer, and which of the two it was

The briefing offered two: either the halftone dictionary's `TransferFunction` changes rendered
colour on a screen and we owe it, or it applies only where the halftone applies and §10.6's
condition carries it. **It is the first**, and the standard draws the line in three places:

- §10.1's list of rendering steps makes only one of the two conditional on the device — "[i]f the
  raster output device supports PDF-defined halftoning, apply halftoning according to 10.6" against
  an unqualified "[f]or any object for which transfer functions are in effect, apply those transfer
  functions";
- §10.6.1 says what a device needing no screen still owes, in the sentence that excuses the screen:
  "[h]alftoning is not required for such devices; after gamma correction by the transfer functions,
  the colour components shall be transmitted directly to the device";
- §10.5's second bullet is a `shall` with no device in it at all.

A halftone dictionary carries two unrelated things — a screen, and a `TransferFunction` — and the
inapplicability this project recorded covers the first. It had been read as covering the dictionary.

## The trap the briefing warned about, sprung and caught

Three shapes of `/HT`, and the round found each of them decides something different: the name
`/Default` **removes** an override rather than leaving one (Table 57 makes it "the halftone that was
in effect at the start of the page", and Table 52 makes that the device's, which specifies no
function in the file); a Type 5 dictionary is read per colourant; anything else governs all three
components. And `TransferFunction /Identity` is an **override**, not a silence — it replaces the
`/TR` in force with the identity, which is what makes `Component` three answers rather than two.

`/TR` and `/HT` are two graphics state parameters in Table 52 and either can be set without the
other, so `TransferState` keeps them apart and composes per component. `Transfer`'s channels became
`Option`s for that composition.

## What the corpus said, and what the instrument said about itself

`examples/transfer_function_census` gained the `/HT` question and asks it **twice** — by the resource
walk it already had, and by a scan of every object the cross-reference table names, descending
through each object's own dictionaries so an inline halftone with no object number is not missed.
**The two disagree over the SafeDocs crawl**: the walk finds `/HT` in fewer documents than the scan
does, because an `/ExtGState` reached only from a pattern's resources or an annotation appearance is
invisible to a page-and-form walk. That is the false zero this tree has produced twice, and it is
caught here only because the count was taken by two instruments. Run the census rather than reading a
number here.

Both agree on the shape: three documents of the whole crawl carry a `TransferFunction`, every
occurrence in either corpus is the name `Identity`, and none of the three also states a `/TR`. So no
corpus page is drawn differently and the four new fixtures are the whole defence (trap 8) — each was
run against the tree that does not read `/HT` and all four fail there (trap 13).

## The erratum that was found by running `emit` first

The obvious authority for "a non-Type-5 halftone governs all components" is §10.6.5.6's sentence
contrasting the two uses of a component dictionary. **Erratum #311 strikes the whole paragraph it is
in**, which `tools/spec-errata emit` printed and `check` did not — the extracted words run together
("graphicsstate", "halftonedictionary") and match nothing. `doc/todo/02` §4's rule that a round
implementing a clause runs `emit` *before* it writes is the reason this was caught before a
quotation of retired text went into the code, the ADR and two ledger rows. The argument now rests on
Table 52's live "[a] halftone screen for gray and colour rendering" and on §10.6.5.6's own opening
about why Type 5 exists.

`doc/errata-read.md`'s two rows for these clauses said "untouched — §10.6 is inapplicable on the
standard's own condition", which was true only while nobody read the clause. Both are corrected.

## The ledger

§10.5 is `implemented`: its one remaining sentence, the `DeviceGray`-into-a-`DeviceCMYK`-device
special case, is conditional on a device this is not, and §11.7.5.2's per-region model is a different
clause's departure and stays that row's. §10.6.5 and its six subclauses went from
`inapplicable` to `implemented`, each note naming the one entry executed and the screen parameters
that are not performed — and quoting the permission §10.6.5.1 states for ignoring them. **`partial`
was this round's first answer and `doc/todo/01`'s fourteenth sweep refused it**, which is that sweep
doing exactly its job: a `partial` row has to name something owed, and here nothing is.
`doc/PLAN.md`'s §10.7.2 precedent is the one that fits — a permission exercised is `implemented`
where there is code to name.
§10.6, §10.6.1 to §10.6.4 stay `inapplicable`; §10.6's note now says what a halftone dictionary
carries that its own condition does not cover, so the row cannot be read as covering it again.

Two stale claims in the code were corrected with it: `pattern.rs` listed `/HT` among the entries a
pattern's `/ExtGState` skips "because this device does not perform them at all", which is now the
wrong reason for the right behaviour — it is skipped for §11.7.5.3's, beside `/TR` and `/TR2` — and
`ext_gstate.rs`'s §8.6.8 comment counted three read entries on that list where there are now four.
