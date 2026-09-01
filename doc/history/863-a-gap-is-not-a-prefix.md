# 863 — A gap is not a prefix: door 2 read against §7.3 and closed

2026-09-01. Argued in [ADR 0787](../adr/0787-a-gap-is-not-a-prefix.md).

Touched: `crates/pdf-syntax/src/parser.rs`, `crates/pdf-model/src/page.rs`,
`crates/pdf-model/tests/damaged_page_dictionaries.rs`, `doc/conformance/ledger.toml`,
`doc/traps/parsers-and-streams.md`, `doc/todo/03-more-corpora.md`,
`doc/adr/0787-a-gap-is-not-a-prefix.md`, `fuzz/corpus/page` (not tracked).

## 1. The reading — `doc/todo/03` section 34's door 2, refused

§34 said the argument "should stop until somebody reads §7.3 properly for it". The reading is done
and the verdict is **no**: no code was written, and the refusal is the round's product.

**§34's own objection turned out to be answerable, which is the part that took the time.** It said a
reader skipping to the next name "has guessed where the bad value ended". Three clauses say
otherwise. §7.2.1 puts tokens *below* objects — bytes "can be grouped into tokens according to the
syntax rules described in subclauses 7.2.2 … through 7.2.4" and "[o]ne or more tokens are assembled
to form higher-level syntactic entities, principally objects". §7.2.3 then gives the failing run one
stated end: "A sequence of consecutive regular characters comprises a single token." And §7.3.1's
nine types, with the introducers §7.3.2 to §7.3.10 each state, make the set of tokens that may
*begin* a value closed — `true`, `false`, `null`, a §7.3.3 digit form, `(`, `<`, `/`, `[`, `<<`, and
§7.3.10's two integers then `R`. A regular run outside that set begins no object of any type, so the
file states no value there at all and there is no extent to guess. Both witnesses fail exactly
there: `R` in `GHOSTSCRIPT-698887-0.pdf`, `\xff` in `GHOSTSCRIPT-699695-1.pdf`.

**What refuses the door is the sentence bounding §7.2.3**: its rules "apply to all characters in the
file except within strings, streams, and comments". A reader knows it is outside those three only by
having tokenised continuously from the object's `<<`. And continuity is what ADR 0784's subset
argument actually rests on — easy to miss, because 0784's sentence is about *order*. The entries are
a subset not because they came first but because each is **the producer's own**, and byte continuity
from a known position is the whole of that proof. Resynchronisation surrenders it by definition.

## 2. The counterexample, one byte wide, and it is pinned

```
A   2 0 obj << /Note (junk /Contents 9 0 R more) /Rotate [0 >] >> endobj
B   2 0 obj << /Note Zjunk /Contents 9 0 R more) /Rotate [0 >] >> endobj
```

`B` is `A` with the literal string's `(` replaced by one regular byte. Under door 2 the prefix
becomes `{/Contents 9 0 R}`; ADR 0786's door fires on it, and object 2 becomes a page drawing object
9 — out of bytes the producer wrote inside a string. **The manufactured entry is not noise a
recovery tolerates: it is the discriminator the recovery acts on**, so one byte decides both that
the object is a page and what it draws. That is trap 5's substitutive direction reached through the
evidence rather than around it.

Three rescues were weighed and each fails, and the second is the sharpest:

- Requiring the resumed reading to reach `>>` makes it *worse* — an object assembled across a gap
  that closes cleanly stops being a `DamagedDictionary` and reaches every reader through
  `Document::get`, since both readings are one function by design.
- Refusing on an unmatched `)` fails because the same damage eats the `)`. `699695-1`'s corruption
  is *runs of `0xFF` over arbitrary bytes* — precisely the mechanism that destroys a `(` — so **the
  witness cannot distinguish its own case from the counterexample**.
- Taking the witnesses' corroboration (their object 4 really is a content stream) is the argument
  about *these files* that §34 had already refused.

Pinned as `damaged_page_dictionaries.rs::the_third_door`, three arms: the intact string, the damaged
one this reader stops at, and the resynchronised reading that manufactures `/Contents`. A future
round that builds the door fails a test instead of rediscovering this on a corpus.

## 3. What moved, and what did not

No behaviour changed and no document renders differently. `GHOSTSCRIPT-698887-0.pdf` and
`GHOSTSCRIPT-699695-1.pdf` stay refused with §7.7.3.2's `/Count` standing, which ADR 0782 already
makes the right answer. The door is named where a round would be standing when it thought of it:
`read_dictionary_body`'s error break, `parse_damaged_dictionary`'s doc comment, and
`PageIdentification`, whose "there is no third variant" paragraph is new.

**The general form is the third sentence in this family**, after ADRs 0343 and 0784: ask what a
prefix of the thing *is*; ask whether the thing's parts are *ordered*; and now **ask what made the
prefix the producer's**. Where the answer is byte continuity from a known position, no recovery may
skip bytes and keep the guarantee. It is in `doc/traps/parsers-and-streams.md` beside the other two.

## 4. Second track — §7.7.3.3 read against the code

The `partial` row for page objects, in the family the primary item is in. Two findings:

- **The note stated one paragraph twice, verbatim** — "Table 31's `/Type` decides one thing in the
  *tree* as well as in the page", once after `/Contents` and once after `/MediaBox`. Removed.
- **Its "genuinely not read" list needed a distinction it did not draw.** ADR 0786 gave `page.rs` a
  `PAGE_ONLY_ENTRIES` array of all 27 of Table 31's page-only keys, so `/LastModified`,
  `/BoxColorInfo`, `/PieceInfo`, `/ID`, `/SeparationInfo`, `/TemplateInstantiated` and the declined
  `/PZ` are now every one of them **named** in `pdf-model`, and named load-bearingly: an entry's
  *presence* decides whether a tree-named damaged object is a page object. That is a use of the key
  and not of the value, and no name-counting sweep can tell them apart — so the row now says
  *values*, and says why.

`/Metadata` was checked and kept: `xmp::Xmp::read` takes any dictionary and would read a page's
packet; its callers are `Xmp::document` and the confined transport, neither of which passes a page.
Status stays `partial` on the seven values.

## 5. The chore — `cargo fuzz cmin page`

Run in the background through the round. **13 319 files / 824 561 236 bytes → 10 826 files /
682 474 771 bytes** — 2493 files and 135 MiB off, keeping 231 875 features and 43 174 coverage
edges, which `cmin`'s own `MERGE-OUTER` lines report and which is the whole distinct-coverage set by
construction. About forty minutes.

**The reduction is far shallower than the five-hundred-and-ninety-third's**, which took the corpus
to about a quarter of its files and a seventh of its bytes, and the reason is the one `doc/todo/02`
§2 already argues rather than a new one: **that session reduced an unreduced corpus and this one
reduced an already-reduced one**. What has been added since is seeds a previous `cmin` would have
kept — real documents with distinct coverage — so 81% of the files survive where 25% did before.
The bullet's warning stands unchanged and the merge is still most of a short run's wall clock.

Trap 24's rule applied: the run is believed on its own `MERGE-OUTER` counters — 13 319 files
processed, 43 174 edges — and on the before/after file counts, never on its exit status.

## 6. Gates

The full sequence, run after the `cmin` finished so that no gate spawning a reference renderer was
measured against a loaded machine (§2's own rule). No new parse path was added — the round's Rust
changes are comments and tests — so no targeted fuzz run was owed beyond the chore. §4's sweeps were
not run: the round adds no verb.
