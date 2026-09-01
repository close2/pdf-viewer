# 0784 — A prefix of a dictionary is a subset, not a dictionary

Session 860. Status: **accepted**.

## Context

ADR 0782 ended by separating two defects that one instrument had called one. `doc/todo/03` §31
named four documents whose page is unreachable; a byte search for `/Type /Page` had found the
declaration in all four and could not tell *a page object the tree cannot reach* from *a page
object nothing can parse*. One of the four — a page tree node that is its own child — was
recovered there. The other three were left with a question attached:

> the question it would ask is whether §7.3.7's dictionary has a prefix worth drawing, which is
> trap 5's test again on a third population.

This is that question. ADR 0343 answered it for a content stream and refused it for a font
program; ADR 0356 sharpened the test and answered it for an image and refused it for a sampled
function; ADR 0359 carried the content-stream half to the four other objects §7.8.2 names. None
of them had asked it of §7.3.7, which is the clause every one of those objects is described by.

## The population, measured before anything was decided

`crates/pdf-model/examples/standing_count_census.rs`, over the Tika issue-tracker corpus —
`batch1`, `batch2`, `batch3` and `batch6`, **16 956 documents, 16 818 of which open**. Its
predicate is deliberately this reader's, which is trap 8's one permitted shape: *a positive
`Pages::len()` over a `get(0)` of `None`*, which is not a question about the standard but about
what share of the files that exist this program fails on. Its classification underneath is read
off the bytes with `Lexer` and `Parser` and never through `Pages`.

**18 documents, 0.11%.** And they are several defects rather than one:

| cause | documents |
|---|---|
| no object in the file declares `/Type /Page` at all | 11 |
| an object declares one and its dictionary opens and then stops | 6 |
| an object declares one and the `obj` keyword has a regular byte glued to it | 1 |

The 6 hold **7 damaged page dictionaries between them, carrying 26 complete entries before the
damage**, four of them already past `/Contents`. Four of the six are one bug report's near
identical attachments.

The sharpest is `batch2/GHOSTSCRIPT/GHOSTSCRIPT-701034-0.pdf`, whose object 2 is

```
2 0 obj
<< /Type /Page /Parent 3 0 R /Resources 6 0 R /Contents 4 0 R /MediaBox [0 0 292 3 >]
/Rotate 0 >>
```

— one byte, `>` where a digit belongs, costing a nine-page document every page it has.

## The reading

### 1. §7.3.7 states no extent, and states that the order is not information

ADR 0356's first question is whether the standard states the thing's *extent* independently of
the bytes that carry it. §7.3.8.2 does for a stream and §7.10.2 does for a sample array. §7.3.7
does not:

> A dictionary shall be written as a sequence of key-value pairs enclosed in double angle
> brackets

and "A dictionary may have zero entries", so no arity can be inferred either. A reader that never
reaches the closing `>>` does not know how many entries the producer wrote.

Its second question is what a prefix of the thing *is*. §7.8.2 makes a content stream "a sequence
of instructions", where order is the meaning and a prefix is a shorter sequence of the same kind.
§7.3.7 says the opposite in as many words:

> The entries in a dictionary represent an associative table and as such shall be unordered even
> though an arbitrary order may be imposed upon them when written in a file. That ordering shall
> be ignored.

So the entries read whole before the damage are **not "the dictionary"**. Two files stating the
same dictionary in two orders, damaged at the same byte, yield different sets. What they *are* is
a **subset of the dictionary's entries, every member of it the producer's own** — which is a
different sentence, and a true one.

Annex C licenses nothing here: it is informative, and its one recovery sentence (§C.4) is about
the cross-reference table. So taking a prefix at all is a **choice**, and this ADR is where it is
documented as one rather than presented as derived (principle 5).

### 2. The two sentences are kept apart by which door a caller comes through

The consequence of §1 is not *never take a prefix*; it is that no code may hold one while
believing it has a dictionary. So:

- `Parser::parse_indirect_object` is **unchanged** and still refuses the whole object. Every
  reader of `Document::get` sees exactly what it saw before — §7.3.10's null — so no reference
  anywhere in the document graph resolves to less of the file than it used to. The recovery is
  *additive*, which is ADR 0782's line and is not relaxed here.
- `Parser::parse_damaged_dictionary` is a second answer a caller asks for **by name**, and it
  answers with a `DamagedDictionary`: the entries, the object, the byte offset at which reading
  stopped, and the error that stopped it. A caller cannot forget what it is holding.
- Both readings are **one function**. `read_dictionary_body` is the only place §7.3.7's body is
  read, so the null rule, the duplicate-key choice and `Limits::max_dict_len` cannot come out
  differently on the two routes. That was the shape of trap 28's failure one level down: two
  claims that were meant to be the same claim.

### 3. Exactly one consumer, and it takes the prefix on the file's own declaration

`pdf_model::Pages`' recovery scan — ADR 0782's, which runs **only where the page tree yields no
page at all** — asks `Document::get` first, as it always did, and asks `Document::damaged_dictionary`
only for an object `get` answered nothing for. It takes what comes back only where the entries
that were whole **themselves state Table 31's `/Type /Page`**. That is the same declaration the
rest of this recovery rests on; a prefix whose damage falls before its own `/Type` says nothing
about what it is, and building a page out of `/Resources` and `/Contents` alone would be the guess
this recovery is defined against.

The residue is named rather than hidden. Every entry the producer wrote after the damage is
missing, and Table 31 hands each absent one a default: no `/Contents` is a page with no marks, no
`/MediaBox` is §7.7.3.4's inheritance and then ADR 0389's chosen sheet, no `/Group` is a page
composited without one. Those are **substitutions**, which is what trap 5 requires a report for.
So the page carries `DictionaryDamage` and `content::interpret` raises
`Unsupported::PageDictionary` before it draws anything — the fifth report in this tree whose
subject is the file, and the second about the page as a whole.

### 4. What was actually blind, and it was not the parser

The recovery could not even *ask* about these objects, and the reason is one line of
`xref::scan_for_objects`: it keeps an offset only where `Parser::parse_indirect_object` succeeded.
So in a file whose cross-reference table had to be rebuilt — which is most of this population —
a damaged object is not merely unparsed but **unnamed**, absent from `object_numbers()` and from
`object_headers()` alike. `Document::damaged_dictionaries` is a second scan over the same
candidate offsets (`xref::object_header_offsets`, now the one place this crate decides what a
header is) and is memoised, lazy, and reached only from a recovery that has already found the
document pageless. It parses no stream data: a damaged dictionary never reaches its `stream`
keyword and a whole one is abandoned at the `>>` this call is not interested in.

It displaces nothing because it decides nothing: it is a statement about the bytes, answered for a
number whether or not something readable also bears it, and the consumer asks `get` first.

## What it recovers, and what it does not

All **6** of the population's damaged-dictionary documents now produce a page and say why; the
census's 18 falls to **12**. Three are pinned in `doc/checks/fixed-documents.toml`, chosen to
include the least flattering:

| document | prefix | what it draws |
|---|---|---|
| `GHOSTSCRIPT-701034-0.pdf` | 4 entries incl. `/Contents` | the page, on the rectangle its `/Parent` states; its `/Contents` stream is *itself* damaged, so both sentences are said |
| `poppler-742-0.pdf` | 7 entries incl. `/MediaBox` | the producer's own sheet, blank, because `/Contents` is among the entries the damage took |
| `poppler-750-0.tgz-0.pdf` | 1 entry — `/Type` alone | this reader's default sheet with nothing on it, and **both** reports fire |

The third is pinned deliberately. It is the weakest thing this change produces, and the argument
for it is not that the page is good but that the alternative — a document reporting two pages and
showing neither — has nowhere to put a report at all, which is ADR 0782 §4's own reasoning one
step along.

**`PDFBOX-4339-0.pdf` stays refused, and it is the boundary.** Its object 3 reads
`3 0 obj\xbc</Type/Page/MediaBox[0 0 3 3]>>`: §7.2.3 makes `\xbc` a regular character, so the
lexer's keyword run is `obj\xbc` and §7.3.10's header does not lex at all — and the single `<`
that follows opens a hexadecimal string rather than a dictionary. There is no object there to
take a prefix of. Reading it would mean deciding that `\xbc<` was meant to be `<<`, which is a
guess about what the producer intended rather than a reading of what the file states.

## The fixtures, which trap 28 is what asks for

`crates/pdf-model/tests/damaged_page_dictionaries.rs`, six pairs on the discipline
`page_tree_nodes.rs` established — one file differing from its partner in one thing:

- an intact object under a working tree reports no damage (the control);
- the same object with one byte changed is read as far as it states, and inherits its
  `/MediaBox` up its own `/Parent`;
- **the guard's own pair**: a tree that *does* reach a page keeps its page, with a damaged
  page-declaring object sitting in the same file;
- a prefix whose damage falls before its `/Type` recovers nothing, and `/Count` stands;
- the damaged object is still `Object::Null` through `Document::get`, which is the additive
  claim made checkable;
- and the report reaches the page's interpretation with the entry count in it.

Trap 28's sentence is *a recovery's guard states when the recovery is needed, its comment states
when the recovery is right, and the round that writes one owes the file where those two disagree.*
Here the guard is "the tree yielded no page" and the rightness condition is "the prefix declares
itself a page", and the third and fourth fixtures are the two files where they come apart.

## What it costs

Nothing at all for a document whose page tree works: `damaged_dictionaries` is reached only from
`scan_for_pages`, which is reached only from `Pages::new`'s `recovering` arm. For a document in
that arm it is one linear pass over the file's `obj` candidates, memoised on the `Document`,
beside the pass `scan_for_pages` already makes through `Document::get`. Every ratcheted gate
prints what it printed before, which is the empirical half: the 974 documents of `doc/pdf.js` hold
no document in this arm that a damaged object could add a page to.
