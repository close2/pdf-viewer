# 860 — A prefix of a dictionary is a subset, not a dictionary

2026-09-01. Argued in [ADR 0784](../adr/0784-a-prefix-of-a-dictionary.md).

**The finding**: a page object whose *own dictionary* is damaged was refused whole, so the page
tree yielded nothing, the recovery scan found nothing declaring itself a page, and the document
opened with a page count and showed nothing. §7.3.7 decides what may be done about that, and it
decides it in a third way — neither *draw the prefix* nor *refuse it*, but *the prefix is a
**subset**, and the two sentences are kept apart by which door a caller comes through*.

Touched: `crates/pdf-syntax/src/parser.rs`, `crates/pdf-syntax/src/document.rs`,
`crates/pdf-syntax/src/xref.rs`, `crates/pdf-syntax/src/lib.rs`,
`crates/pdf-model/src/page.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/content/report.rs`, `crates/viewer-core/src/report.rs`,
`crates/pdf-model/tests/damaged_page_dictionaries.rs` (new),
`crates/pdf-model/tests/corpus.rs`, `crates/pdf-model/tests/oracle.rs`,
`crates/pdf-model/examples/standing_count_census.rs` (new),
`doc/conformance/ledger.toml`, `doc/checks/fixed-documents.toml`,
`doc/traps/parsers-and-streams.md`, `doc/todo/03-more-corpora.md`,
`doc/QUORRA_FEEDBACK.md`, `doc/adr/0784-a-prefix-of-a-dictionary.md`.

## The population, measured before anything was decided

`examples/standing_count_census`, over the whole Tika issue-tracker corpus — `batch1`, `batch2`,
`batch3`, `batch6`. Its predicate is deliberately this reader's, which is trap 8's one permitted
shape (the question is what share of the files that exist this program fails on, and only the
program can answer it); the classification underneath is read off the bytes with `Lexer` and
`Parser`, never through `Pages`. Run it for the numbers; what belongs here is the shape.

**It is several defects, not one.** The great majority of the documents whose `/Count` stands over
no page declare `/Type /Page` **nowhere in the file** — for those, ADR 0782's standing count and a
refusal out loud is already the right answer and there is nothing to recover from. A minority hold
a page object whose dictionary opens and then stops. Exactly one holds an object whose `obj`
keyword has a regular byte glued to it.

`batch2` was fetched by round 859 and extracted here, verified against Apache's published SHA-512;
`batch4`'s fetch was running throughout and was left alone.

## The reading

ADR 0784 has it. In one paragraph: ADR 0356's first question is whether the standard states the
thing's extent, and §7.3.7 does not — a dictionary's extent is its closing `>>` and nothing else,
and "A dictionary may have zero entries" rules out inferring an arity. Its second question is what
a prefix *is*, and §7.3.7 answers it against the prefix: "[t]he entries in a dictionary represent
an associative table and as such shall be unordered … That ordering shall be ignored." Two files
stating one dictionary in two orders, damaged at the same byte, yield different sets — so the
entries read whole are **not the dictionary**. They are a subset of it, every member the
producer's own, which is a different and true sentence.

The design is that no code may hold one while believing the other. `Document::get` is unchanged
and still refuses the whole object, so nothing in the document graph reads less of the file than
it did; `Parser::parse_damaged_dictionary` is a second door opened by name, handing back the byte
offset the reading stopped at; both readings are **one function**, so §7.3.7's null rule, its
duplicate key and `max_dict_len` cannot disagree between them. One consumer: `Pages`' recovery,
which runs only where the tree yields no page and takes a prefix only where the entries that were
whole *themselves* state Table 31's `/Type /Page`.

The residue is reported. Every entry after the damage is read as one of Table 31's defaults, which
is a substitution, so the page carries `DictionaryDamage` and `interpret` raises
`Unsupported::PageDictionary` — the fifth report in this tree whose subject is the file, the
fourteenth place it reports while drawing.

## What was actually blind, and it was not the parser

`xref::scan_for_objects` keeps an offset only where `parse_indirect_object` succeeded. So in a
file whose cross-reference table had to be rebuilt — which is most of this population — a damaged
object is not merely unparsed but **unnamed**: absent from `object_numbers()` and from
`object_headers()` alike, so the recovery could not even ask about it. That is the half of the
defect nothing in the tree could have reported, and it is the half that generalises: anything else
wanting to ask a question about an object that will not parse has the same problem.
`Document::damaged_dictionaries` is a second, memoised scan over the same candidate offsets, and
`xref::object_header_offsets` is now the one place this crate decides what a header is.

## Per witness

- `GHOSTSCRIPT-701034-0.pdf` (and its three near-identical siblings): one byte, `>` where a digit
  belongs inside `/MediaBox`, cost a nine-page document every page. Four entries whole including
  `/Contents`; the page draws on the rectangle its `/Parent` states, and its content stream is
  *itself* damaged, so both sentences are said.
- `poppler-742-0.pdf`: seven entries whole including `/MediaBox`; the producer's own sheet, blank,
  because `/Contents` is among the entries the damage took.
- `poppler-750-0.tgz-0.pdf`: one entry — `/Type` alone — so this reader's default sheet with
  nothing on it, and **both** reports fire. Pinned deliberately as the least flattering of the
  three: the argument for it is not that the page is good but that a document reporting two pages
  and showing neither has nowhere to put a report at all.
- `PDFBOX-4339-0.pdf`: **still refused, and now for a stated reason.** `3 0 obj\xbc<<` — §7.2.3
  makes `\xbc` a regular character, so the lexer's keyword run is `obj\xbc` and §7.3.10's header
  does not lex; the single `<` that follows opens a hexadecimal string. There is no object there
  to take a prefix of, and reading one would mean deciding that `\xbc<` was meant to be `<<`.

## The one thing the round did not expect

**`doc/pdf.js` has a witness too**, and the oracle found it rather than the census:
`poppler-742-0-fuzzed.pdf` is the same defect in the same file under another name, and it left
`NO_RENDER_NO_PAGE_IN_THE_TREE` for `NOT_COMPARABLE_NO_REFERENCE_REACHES_A_PAGE`. The three
references were asked again rather than quoted — `pdftoppm` writes no file, `mutool` repairs the
table and writes zero bytes, `gs` says *Catalog dictionary not located in file* — which is
agreement about the **object** and not about the page: the seven entries are in the file and every
one of the four readers can see them.

## Second track

`--bin owed`'s next row, §14.8.6, read and **kept**. What the reading adds is the name of the
requirement this subclause addresses to a *processor* and which the note did not carry: §14.8.6.1's
"[w]hen a namespace is not explicitly specified … it shall be assumed to be within this default
standard structure namespace" is `Tree::namespace`'s answer for an element with no `/NS`, asserted
inside `a_namespaces_own_role_map_is_the_one_that_applies` against a fixture whose two role maps
disagree on purpose. So every sentence of the subclause addressed to a reader is done, and
`partial` now stands for exactly one thing — nothing reports the tagged document whose elements end
in no standard namespace, which the clause makes a requirement on the *file*. The row stays
`partial` rather than following §7.3.7's precedent of leaving a file-addressed `shall` on an
`implemented` row, because both children record the same unreported violation and a parent settled
over two unsettled children is what the ledger's own arithmetic sweep prints.

§7.3.7 and §7.7.3.2 both gained the reading above; both were and remain non-`unreviewed`.

## The warm-up, which turned into a quorra report

`render-quorra` on `MOZILLA-831621-14.pdf` — round 859's soft-mask witness — **refuses the frame
before drawing anything**, at 3 138 560 000 scene-derived bytes against the 256 MiB default budget.
The arithmetic is the finding: 3 138 560 000 / (1280 × 800) is 3065.0 exactly, one page-sized
allocation apiece for the page's 3059 soft masks, whose consumers between them can read 0.1% of the
page. So quorra's encoder pays the cost ADR 0783 stopped our CPU backend paying, and pays it in its
own pricing rather than in time. Written up as `doc/QUORRA_FEEDBACK.md` §42, as a report about the
estimate and not as an ask for a larger budget — the same position as §40.
