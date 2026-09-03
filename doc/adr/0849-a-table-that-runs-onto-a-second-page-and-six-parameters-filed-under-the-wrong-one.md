# 0849 — A table that runs onto a second page, and six parameters filed under the wrong one

Date: 2026-09-03. Session 903.
Status: accepted.
Clauses: ISO 32000-2 §8.4.1, and every clause that cites a graphics state parameter's initial
value — §8.4.3.2, §8.6.5.8, §10.7.5, §11.6.4.3, §11.7.5.2.
Corrects ADR 0620 §2's second finding, which named the right shape and put one of its own examples
on the wrong side of the line.

## What is true

§8.4.1 has two tables. **Table 51 — Device-independent graphics state parameters** runs from CTM
to `rendering intent` on ISO 32000-2:2020 page 157, **continues onto page 158 with its own
`Parameter | Type | Value` header and no caption of its own**, and only then does page 159 carry
**Table 52 — Device-dependent graphics state parameters**, whose rows are `overprint`,
`overprint mode`, `black generation`, `undercolor removal`, `transfer`, `halftone`, `flatness` and
`smoothness`.

The six rows on that unlabelled continuation page are:

**stroke adjustment, blend mode, soft mask, alpha constant, alpha source, black point compensation.**

All six are Table 51's — **device-independent** — and the first of them says so in its own NOTE:

> This is considered a device-independent parameter, even though the details of its effects are
> device-dependent.

That NOTE only makes sense where it stands. In Table 52 it would be arguing against the table it is
in; in Table 51 it explains why a parameter with device-dependent effects is listed among the
device-independent ones, which is exactly the question a reader would have.

Checked twice and against two instruments, because a table boundary is precisely what a conversion
can get wrong: `doc/md/ISO_32000-2_sponsored_EC3.md` renders the continuation as an uncaptioned
table between the two captions, and `pdftotext -layout -f 173 -l 173` over `doc/
ISO_32000-2_sponsored_EC3.pdf` shows page 158 whole — a bare column header, six rows,
`stroke adjustment` first, no caption anywhere on the page.

## What this tree said

Ten citations in `crates/`, two in `doc/todo/`, one in the ledger and four in `doc/adr/` attributed
one of those six parameters to Table 52. Corrected here:

| | said | is |
|---|---|---|
| `pdf-model/src/content.rs`, the rendering intent field | Table 52 | Table 51 |
| the same file, `alpha_is_shape` and its seed | Table 52, twice | Table 51 |
| the same file, a pattern's initial state | "Table 52's initial values" | Table 51's **and** Table 52's |
| `pdf-model/src/content/transparency.rs`, the two alpha constants | Table 52 | Table 51 |
| `pdf-model/src/content/pattern.rs`, the same pair | Table 52 | Table 51 |
| `pdf-render/src/sub_pixel.rs`, twice, the stroke adjustment parameter | Table 52 | Table 51 |
| `pdf-model/tests/oracle.rs`, twice, the same parameter | Table 52 | Table 51 |
| `pdf-model/tests/oracle.rs`, `AMBIGUOUS_STROKE_ADJUSTMENT`'s removal experiment | **Table 58** | Table 51 |
| `doc/conformance/ledger.toml` §8.4.3.2 | Table 52 | Table 51 |
| `doc/todo/_scan-conversion.md`, `doc/todo/11` | Table 52 | Table 51 |

The `oracle.rs` one that said **Table 58** is worth separating from the rest: Table 58 is the path
construction operators, and the sentence was the control for ADR 0688's removal experiment — *Table
58's initial value for `SA` is `false`, so the page is the same page with stroke adjustment
disabled*. The experiment is right and the citation under it named a table with no parameters in it
at all. §10.7.5's own ledger row had already been corrected once for exactly this number, in the
three-hundred-and-eighty-ninth session, and the correction reached the row and not the note that
depends on it — ADR 0101's shape, which this project has now recorded four times.

**Four `doc/adr/` files carry it and are amended by this one rather than edited**, per ADR 0232 §2:
[0419](0419-four-renderers-four-floors-and-the-clause-states-one-of-them.md) §1 and
[0420](0420-the-mark-a-placement-took-after-the-alpha-survived.md) §5 give the stroke adjustment
parameter's initial value to Table 52; [0165](0165-a-clip-on-the-edge-of-what-it-clips.md) gives
Table 52 the default line width, which is Table 51's; and
[0620](0620-the-entry-a-list-audited-three-times-never-named.md) §2 — the ADR that *found* this
shape — writes "Table 52 is the device-*dependent* list — flatness, smoothness, overprint, stroke
adjustment", which is right about three of the four.

## Why it went unseen, and it is 0620's own answer with a fifth instance under it

ADR 0620 §3 asked why no sweep prints a wrong table number and answered it for the rendering intent:
`--bin tables` verifies a `Table NNN` citation's `/Key` attributions against the entries the
standard puts in that table, and **the thing being attributed here is not a `/Key`**. Table 51 and
Table 52 have no keys at all: their rows are prose parameter names — *stroke adjustment*, *alpha
source*, *line width* — so every citation of either lands in the sweep's keyless count, where a
wrong number is indistinguishable from a right one. The checker in `tools/conformance` confirms
the table *exists* and prints its title, which for `Table 52` it does, correctly, every time.

0620 recorded that as one instance and did not ask how many siblings it had. It had ten, all of
them about parameters on the same physical page of the standard, and the mechanism that hid them is
the one that made them: **the tables meet on a page with no caption**, so a reader looking for
`stroke adjustment` finds the nearest caption below it and takes that.

**The grep that finds the family** costs a second and is stated here rather than built into a sweep,
because it is a fixed list of six words against one number:

```sh
grep -rn "Table 52" crates/ tools/ fuzz/ doc/ --include=*.rs --include=*.md --include=*.toml |
  grep -E "stroke adjust|blend mode|soft mask|alpha constant|alpha source|black point"
```

It is not made a sweep for the reason 0620 gives about `--bin tables` and one more: the population
is closed. Six parameter names, one wrong number, and a `Table 51` citation for a Table 52
parameter is the mirror image the same grep finds with the lists exchanged.

## What it changes, which is one reading and no pixel

No code path branches on which table a parameter is in, so nothing this tree draws moves. Two
readings do:

- **§8.4.1's advice reaches fewer parameters than this tree believed.** "[A] page description that
  is intended to be device-independent should not be written to modify these parameters" is said of
  Table 52's list. A producer writing `/SA`, `/BM`, `/SMask`, `/CA`, `/ca` or `/AIS` is not writing
  a device-dependent page description, and this tree had six parameters filed under a `should` that
  is not about them.
- **§8.7.3.1's prohibition on a pattern reaches fewer too.** It says a pattern's content stream
  shall not set any of the device-dependent graphics state parameters, and points at Table 52 for
  which those are. (Not quoted: `doc/md/`'s conversion breaks the word as *device -dependent*, and
  a quotation this tree cannot verify against its own instrument is prose.) A tiling
  pattern's cell **may** state `/SA`, a blend mode, a soft mask and the alpha constants; only
  overprint, the two colour-removal functions, the transfer function, the halftone, flatness and
  smoothness are forbidden it. That is a requirement on a *producer* and there is nothing for a
  reader to enforce, so no code is owed — but the sentence had a wrong six-parameter shadow, and
  §8.6.8's separate list for an uncoloured cell (`TR`, `TR2`, `BG`, `BG2`, `UCR`, `UCR2`, `HT`,
  `UseBlackPtComp`, which `ext_gstate.rs` already honours) is unaffected and is drawn from a clause
  that names its entries rather than a table.

## Consequences

- A round citing a graphics state parameter's initial value cites **Table 51** unless the parameter
  is one of Table 52's eight, and Table 52's eight are worth knowing by name because the list is
  short: overprint, overprint mode, black generation, undercolour removal, transfer, halftone,
  flatness, smoothness.
- `doc/conformance/ledger.toml`'s §8.4 row already lists Table 52's set correctly and its §8.4.1 row
  already says "Table 51's initial values", and
  `line_parameters.rs::the_initial_values_are_table_51s` asserts three of them by that name. The
  ledger had the number right and the code depending on it did not, which is 0620's sentence about
  `icc.rs` one clause family over.
