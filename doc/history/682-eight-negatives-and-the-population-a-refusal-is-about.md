# 682 — Eight negatives, and the population a refusal is about

Eight of `doc/todo/01`'s owed negatives re-derived over the SafeDocs crawl, **seven of them false**
— and the one that mattered most was false by 2882 documents while the refusal it justifies is
reached by six, which is trap 11's rule pointed at a census instead of at a report.

Date: 2026-08-23.
ADR: [0516](../adr/0516-eight-negatives-re-derived-and-the-population-a-refusal-is-about.md).

Touched: `crates/pdf-model/examples/absence_audit.rs`, `crates/pdf-model/src/image.rs` (one doc
comment), `crates/pdf-model/tests/transparency_groups.rs` (one doc comment),
`doc/conformance/ledger.toml` (§7.6.5, §7.9.2.2.2, §8.9.5.2, §8.10.3, §11.6.5.2, §12.3.2.2,
§12.4.2, §12.5.1), `doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, the ADR and this
file.

## What the queue said

`doc/todo/01`'s own script, run in this worktree before the edit, printed **18 done and 28 owed** —
one more `done` than the briefing quoted, because the six-hundred-and-seventy-sixth's own row had
landed since. After the edit it prints **26 and 20**. The eight rows moved are the second of the
four groups, which the file said needed a structural block in `absence_audit`.

The ninth row in that group, §14.8.2.5.3, was **moved rather than measured**: `/ReversedChars` is a
marked-content tag inside a content stream, so a block over the object graph would have reported a
false zero for it. It belongs with the five claims that need a content-stream census, and the todo
file says so now.

## The eight, both populations

Curated is 1251 documents, `CC-MAIN-2021-31` is 65 944, stated apart per ADR 0490.

| clause | curated | crawl |
|---|---|---|
| §7.6.5 public-key `/Filter` | 0 | **1** |
| §7.9.2.2.2 U+001B escape | 0 | **1**, a *lone* escape |
| §8.9.5.2 a general `/Decode` | 0 | **7** |
| §8.10.3 a `/Group` that is not `/Transparency` | 0 | **0** |
| §11.6.5.2 a codec-carrying `/SMask` | **6** | **2882** |
| §11.6.5.2, on a pair the deferred route would take | **0** | **6** |
| §12.3.2.2 an integer first element | **5**, all remote | **599**, 44 of them local |
| §12.4.2 all three of the example's ranges | **1** | **11** of 722 stating ≥3 |
| §12.5.1 a rotated page with a widget | 0 | **15** |

## Three things worth carrying forward

**A false negative does not imply owed work.** §7.6.5's one witness is declined by name with
`SyntaxError::UnsupportedEncryption`, which is the row's own sentence working. Ask what the code
does when the document arrives; that is a different question from the count and it is the one that
decides.

**Count the condition, not the noun.** §11.6.5.2's residue is a mask behind an image codec, and
`soft_mask_entry` asks about the codec only where `worth_combining` has already refused the finer
grid. A block firing on "a codec-carrying `/SMask`" measures 2882 documents; a block firing on the
condition the sentence states measures six. The block is two blocks now.

**Probe a positive as well as a zero.** The planted witness caught two false zeros, as the recipe
promises. The *other* direction caught a false hit that nobody had a recipe for:
`issue10339_reduced.pdf`'s `/Decode [255.0 0.0]` on an eight-bit `Indexed` image is Table 88's own
default reversed, not a departure — so the first draft of that block would have retired a claim
that holds.

## Also

`spec-errata emit` on every clause the round touched found **Issue #536** on §12.3.2.2, never
recorded: a Caret with no `StrikeOut`, so the twelfth sweep is blind to it, inserting "interactive
processors should" before Table 149's `/FitR` imperative "use the smaller of the two". Nothing here
changes — `apply_view` already does — but the rule moved from an instruction to a `should`, and
`doc/errata-read.md` records it. That is the third instance of the no-`StrikeOut` shape in that
file, and the second consecutive round to find an erratum by opening the page of a row it was
already editing.

## Gates and sweeps

The change is an example, two doc comments, the ledger and four documents, so `doc/todo/02` §2's
core ran whole — `fmt`, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`,
`nextest run --workspace`, `test --workspace --doc`, `check` over `fuzz/` — plus
`cargo test -p conformance`. All green; the quotation gate failed once first, on two sentences of
§7.9.2.2.**1** quoted under a §7.9.2.2.**2** citation, and the fix was to cite the clause the
sentences are in.

Fourteen sweeps run before and after, from this worktree with `cargo run` rather than from a
build directory (trap 15). **The `tables` sweep paid on this round's own new comments**, which is
why its "absent" count is where it started: `absence_audit` said Table **96** for the group
attributes dictionary, which is Table **94** — 96 is optional content groups — and Table **173**
for a link's `/Dest`, which is Table **176**'s, with an outline item's being Table **151**'s. Both
fixed before the commit.

Deltas, before → after, and each accounted for: `counts` 6979 → 6992 sentences and 380 → 381
attributed, the one new attribution being under a clause with no rows below it; `quotations`
5456 → 5468 over the documents with verbatim 2377 → 2380, and 1835 → 1837 in the ledger with both
new ones unrelated to a specification and none diverging; `tables` 5941 → 5958 sentences and
2245 → 2250 citations, +3 agreeing and +2 under a table the conversion gives no entries;
`pointers` 7276 → 7300 paths and 109 → 111 symbols, none newly absent or undefined; `owed`
3561 → 3583 terms with its 182 unnamed over 114 rows unchanged; `entries` 285 → 286 rows and
827 → 831 entries — the new row is §12.4.2, which entered the population because its note now
names an arrival, and the entry it does not name is Table 161's optional `/Type`, which nothing
reads and which is not a debt; `overtaken` 502 → 503 decision records, this round's ADR.
`overstated`, `blockers`, `callers`, `capabilities`, `inapplicable`, `retired` and `unread` did
not move at all.
