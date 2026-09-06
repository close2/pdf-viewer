# 0912 — Five answers to one clause, four families, and a rule wrong on its own second witness

Session 936. Status: **accepted**. Amends ADR 0904's truncating half; ADR 0913 is the code that
follows from this reading.

## Context

ADR 0904 opened with the sentence "one answer to one clause rather than two". This round swept the
class it belongs to — every place in `crates/pdf-model/src` and `crates/pdf-syntax/src` that reads
an integer out of an object — and found the tree giving that one clause **five** answers, of which
two were ADRs and three were nobody's decision in particular.

§7.3.3 is the clause:

> A real number shall not be present when an integer is expected.

The `shall not` is the writer's. Nothing anywhere addresses the reader, so every answer is a
choice in the sense `CLAUDE.md` principle 5 means.

## The population, derived rather than listed

`grep -rn as_integer crates/pdf-model/src crates/pdf-syntax/src` is 133 lines. Twenty-one are not
reads of a document at all — `pdf_syntax::write`'s own `as_integer(u64) -> i64`, the type 4 stack's
`Value::as_integer`, three doc comments and four test assertions — leaving **112 sites** that read
an integer out of a PDF object, in 41 files.

The *world's* half of the population is not greppable, so it was measured:
`crates/pdf-model/examples/integer_entry_census.rs` derives, from the Arlington model, every key
name typed `integer` or `bitmask` in some table and `number` in none — 117 names — and counts what
stands at each over every corpus on this disk. It walks two things, because an inline image is not
an object and the only witness this project has is one: every object of every cross-reference
table, and every `BI` … `ID` dictionary in every stream a lexer is ever pointed at, the second read
with the census's own tokens rather than by asking `pdf_model::inline_image` (trap 8). It was run
against the known witness before it was believed, and finds it.

## The five answers

| where | the rule | recorded as |
|---|---|---|
| `pdf_model::function`, §7.10.5's calculator | **truncate** | ADR 0371 |
| `pdf_model::image`, Table 87's `/Width`, `/Height` | **truncate** | ADR 0904 |
| `pdf_model::content::run::integer_at`, a content-stream operand code | accept a real that **is integral and under 1000** | a doc comment |
| `pdf_model::signature`, §12.8.2.2's `/DocMDP` `/P` | **refuse**, and take the table's default | a test's doc comment |
| `pdf_font::substitute`, Table 122's `/FontWeight` | read it as a **number** and never convert | a comment calling itself "a reader's tolerance" |

A sixth is in the lexer and is a different question: `salvage_number` turns a *malformed* run with
no fractional part into `Token::Integer`, which is §7.3.3's other departure and ADR 0303's.

Two of those five are not defects, which is the finding that makes the rest legible. The refusal in
`signature` is **right**, and for a reason that outranks any reading of §7.3.3: `CLAUDE.md`
principle 3 makes a document's restrictions the reader's to set, so a malformed restriction must
never bind *harder* than the default. The `/FontWeight` reading is right too, and for a reason that
dissolves the question: the value is only ever compared against a threshold, so nothing has to
become an integer at all.

## The four families, and the argument for each

A real arriving where an integer is typed carries one of two defects and a reader cannot tell them
apart from the value: an integer *value* in real *syntax* (`1062.00`), or a genuinely fractional
value (`1062.5`). What decides the right rule is not the value but **what the integer is for**.

1. **A magnitude on a grid the rest of the file corroborates** — Table 87's `/Width` and
   `/Height`, and only those two so far. The integers are amounts; a neighbouring one is nearly the
   stated value; and the sample data, §7.4.8's codestream frame and the length check are all second
   opinions that catch a wrong reading. **Read it.**
2. **An identifier wearing a number** — a bit set (`/F`, `/Ff`, `/P`, `/SigFlags`), an enumeration
   (`/PatternType`, `/ShadingType`, `/LC`, `/LJ`, `/Q`), a key into a number tree
   (`/StructParent`, `/StructParents`, `/MCID`), an object or generation number. The integers here
   are **names**. A value between two of them is not *nearly* either; the nearest one is a
   different name rather than a weaker version of what was stated, and the consequence of picking
   it is silent and unbounded — the wrong structure element, the wrong permission set, the wrong
   pattern type. Arlington draws this line itself, typing a flag word `bitmask` rather than
   `integer` and glossing it "an integer used as a set of flags". **Refuse, and take the entry's
   stated default**, which is the reading the tree already has for an entry that is absent.
3. **A claim about the file that another reading settles** — §7.7.3.2's `/Count`, §7.5.8's
   `/Size`, a stream's `/Length`. Here the standard itself ranks the two readings, in `/Count`'s
   own Table 30 cell: the `/Kids` arrays are what "definitively determines the number of descendant
   pages", and the entry is called "redundant". Accepting a malformed value would make a damaged
   file agree with itself and **suppress the disagreement that is the finding**. `Pages::new`
   already falls through to `count_leaves` where `/Count` is implausible, and `parser.rs` already
   scans for `endstream` where `/Length` is; a tolerance here would buy a worse answer faster.
   **Refuse, and derive it.**
4. **A restriction the document asserts over its reader** — Table 22's `/P`, `/DocMDP`'s `/P`,
   §12.8.6's usage rights. **Refuse, in the permissive direction**, which `signature` does and
   principle 3 requires.

## What the world contains

Over **90 128 documents opened of 90 537 paths** — `doc/pdf.js`, the four `doc/corpora`
submodules, `openpreserve`, the tika issue trackers and the SafeDocs crawl — at the 117 key names
the model types integer and never number:

| | |
|---|---:|
| entries holding an integer | 29 066 528 |
| entries holding a **real** | **443** |
| of those, with no fractional part | 428 |
| documents carrying at least one | **212** |
| inline image entries holding an integer | 15 583 176 |

and by key, which is where the shape is:

| key | entries | documents | family | what this tree does |
|---|---:|---:|---|---|
| `/FontWeight` | 322 | 186 | fifth shape | already read as a number, against a threshold |
| `/Count` | 35 | 3 | 3 | refused; the tree walks the `/Kids` and is right to |
| `/FormType` | 27 | 6 | 2 | not read at all; Table 95 says the value "shall be 1" and all 27 are `1.0` |
| `/Order` | 15 | 3 | — | **not Table 38's**: the model types `/Order` an integer only on a sampled function, and every witness is a `/Collection` schema field's — the same name-only cost as `/Q` |
| `/OPM` | 13 | 2 | 2 | not read: Table 58's `/OP`, `/op` and `/OPM` are deliberately unimplemented |
| `/Q` | 11 | 2 | — | **not Table 232's**: every one is inside a `/PieceInfo` private dictionary, which is the cost of a name-only predicate and is stated as such |
| `/Rotate` | 9 | 3 | 1 or 2 | refused; `doc/questions/Q39` |
| `/Length` | 5 | 5 | 3 | refused; the scan for `endstream` is the better answer |
| `/Columns` | 2 | 2 | 1 | refused for range — both values are past 10²⁹ |
| `/Width`, `/Height` **inline** | 4 | 2 | 1 | read (ADR 0904, and this round) |

Fifteen further key names are typed `integer` in one table and `number` in another — `/Width` and
`/Height` among them, because §14.8.5.4's layout attribute of the same name is a number — and the
census **counts them separately and does not judge them**, printing witnesses so the reading is a
person's. Every witness printed is a structure element's layout attribute or a `PaperMetaData`
entry, and the reach measurement below settles the rest without inspection: all 332 documents with
an object-level real `/Width` and all 380 with a real `/Height` were digested before and after, and
**not one of them moved**, which is what proves none was an image `XObject`. Both documents in the
world that write a Table 87 dimension as a real write it inside a `BI`.

## The rule was wrong, and its own second witness says so

ADR 0904 chose **truncation**, on ADR 0371's word, and pinned it with a `/W 2.9` fixture. It also
measured its reach correctly and read the result incompletely: two documents changed, and the
second one's new report was recorded as "ADR 0799's reading arriving one clause later" rather than
as an accusation.

`GHOSTSCRIPT-695872-0.pdf` writes `/W 737.999999999715 /H 49.999999999` over a JPEG whose own frame
is **738 × 50**. Truncation reads that grid one short in *both* axes, and the page then reports:

```
<inline>: the JPEG frame is 738x50 where the dictionary says 737x49
```

The file did not contradict itself. **We did**, and then said so about the file — which is trap 11
from the other end: a report whose condition is satisfied by the reader's own conversion is a
report about the reader. The nearest integer agrees with the codestream in both axes, agrees with
truncation on all 428 of the 443 reals that have no fractional part, and differs from it nowhere
else in the world.

So the rule for family 1 is **the nearest integer, ties away from zero**, and ADR 0904's truncating
paragraph is superseded. The fixture that pinned it is rewritten with its argument: a hand-made
`/W 2.9` cannot say which rule is right (trap 4), it can only record which was chosen, and under
the new rule it is *reported* — "its samples stop at 12 bytes where 3x3 … needs 27" — where under
truncation it was silent.

**Considered and declined**: accepting only a real that is an integer *within the representation*
(`737.999999999715` as an `f32` is exactly `738.0`), refusing `2.9` outright. It is the more
principled cut and it answers both witnesses, but it needs a threshold for "within", and the only
non-arbitrary one routes through Annex C.2 — which is informative and says only that processors
"often" use IEEE 754 single or double precision. A choice wearing a derivation is worse than a
choice; and the two rules differ only on values no producer writes.

**Also considered**: reporting the repair. A §7.3.3 departure the reader now silently mends is
still the file breaking a `shall not`, and trap 5 asks for loudness. Declined, because the report
would be about input that is no longer unsupported: the page after the repair is the producer's
page, and a report says a page may be wrong. What stays loud is everything the repair does not
reach — a value that names no grid is refused exactly as before.

## The reach, measured before it was believed

`display_list_digest` over 1 353 documents — all of `doc/pdf.js`, plus every document in the world
the census found a real `/Width` or `/Height` in, judged or not — before and after, in one sitting
with one `pdf-sandbox-worker` on disk:

**One line of 1 353 differs, and only its report half.** `GHOSTSCRIPT-695872-0.pdf`: 673 commands
and display-list hash `1e75317464c7b035` on both arms, byte for byte, with `1` report before and
`0` after. The image always decoded on the codestream's grid — §7.4.8's own reading — so the
dimensions never reached a pixel; what moved is the sentence.

That is also why the digest gained a column. A change that moves only what the program *said* was
invisible to it, and this round's is exactly that shape: without the report hash the diff would
have been empty and the fix would have read as a no-op.

## Consequences

- `pdf_model::integer_entry` is the one place the rule lives, and ADR 0913 is what moved into it.
- `crates/pdf-model/examples/integer_entry_census.rs` is the instrument, and it takes
  `--list-keys` (the derived population itself) and `--witnesses N` (the whole document list, for a
  round measuring reach). Its two halves cost about twelve seconds per eight thousand documents; a
  walk over the whole disk at twenty-four threads crosses an 8 GiB `RLIMIT_DATA`, so it shards, and
  one document — a 5.6 GiB attachment in `batch5/poppler` — is not walkable inside the round's
  memory budget at all and is reported rather than skipped quietly.
- The three families that refuse are refusals **with an argument** now, in one module comment, and
  §7.3.3's ledger row carries the same reading.
- `doc/questions/Q39` is the one case the sweep could not settle: `/Rotate`, where a population
  exists and does not discriminate.
