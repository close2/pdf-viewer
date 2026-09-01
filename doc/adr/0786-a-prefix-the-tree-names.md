# 0786 — A prefix the tree names, and a producer's `shall` a reader can still say out loud

Session 862. Status: **accepted**.

Two decisions, and they share a shape rather than a subject: each takes a sentence somebody had
filed as addressed to a producer and asks what a *reader* may honestly do with a file that breaks
it.

## Part 1 — the second door into a damaged page dictionary

### Context

ADR 0784 recovered a page from the readable prefix of a damaged dictionary, and took one only where
the prefix *itself* states Table 31's `/Type /Page`. ADR 0785 then read the eleven documents whose
files declare `/Type /Page` nowhere, split them into five defects, and handed on two doors it did not
open. `doc/todo/03` §34 argues the first of them. This is that door.

The reason 0784's rule was right and is still right where it is: its consumer finds its candidates by
**scanning the whole file**, and an object that says nothing about itself could be anything. The
second door is handed its candidate by the page tree, and that changes what the prefix has to prove.

### The reading, which is the standard's own twice

§7.7.3.2 Table 30's `/Kids` cell:

> An array of indirect references to the immediate children of this node. The children shall only
> be page objects or other page tree nodes.

So a named object is one of exactly two things. The sentence after the same table closes what the
second of them may hold:

> In addition to the entries shown in "Table 30 -Required entries in a page tree node" , a page
> tree node may contain further entries defining inherited attributes for the page objects that are
> its descendants

and §7.7.3.4's inherited attributes are `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate`. A page
tree node's legitimate keys are therefore Table 30's four and those four — a **closed** set, stated
rather than inferred. A prefix carrying `/Contents` or `/Annots` was written by a producer describing
a page object.

**Three things came out of building it that §34's sketch did not have, and each is the difference
between a right answer and a plausible one.**

#### 1. The discriminator is a positive list of Table 31's keys, not the complement of Table 30's

The two sound identical and part company on a real document. `poppler-gitlab/poppler-355-0.pdf`'s
prefix is a garbled key, `/WinAnsiEncope`, `/Parent` and `/CropBox`. `/WinAnsiEncope` is in neither
table, so under *any key a node may not carry* it is evidence and the file is recovered; under *any
key Table 31 defines for a page object* it is evidence of nothing and the file stays refused — which
is what §34 argued for and what the complement would have quietly undone.

The reason is §7.3.7's, and it is the same sentence ADR 0784 rests on: a prefix says what the
producer **did** write and nothing whatever about what it did not. So the evidence has to run from an
entry being *present*, never from a node's entry being *absent* — which is also why "this dictionary
states no `/Kids`" is not a thing this door can know. `PAGE_ONLY_ENTRIES` is Table 31 minus Table 30
minus §7.7.3.4, twenty-seven names, and the fixture with a key in neither table is what holds it
there.

#### 2. The tree is walked from the catalogue's `/Pages`, not from every `/Kids` in the file

`standing_count_census` follows every array called `/Kids` of every object that parses, and may,
because it is a census answering a question about a population. A *reader* that did the same would
collect §12.7.4.2's form-field kids and §7.9.6's name-tree kids, and §7.7.3.2 says nothing at all
about the objects those name.

The obvious tightening — follow the `/Kids` of an object stating `/Type /Pages` — **loses three of
the five witnesses**, which is why it is worth writing down. `GHOSTSCRIPT-698991-0`'s root is
`<< /Kids[42 0 R]/Count 1>>`, `GHOSTSCRIPT-699018-0`'s is `<</Kids [3 0 R] /Count 1 >>`, and
`poppler-192-0`'s is `<< /Count 1 /Kids [ 6 0 R ] /T0 500 500s.>>`: not one of them states a `/Type`.
Table 29 is what makes the walk sound anyway — the catalogue's `/Pages` is "[t]he page tree node that
shall be the root of the document's page tree", so the root is a node **by the catalogue's
declaration**, whatever its own dictionary omits, and everything its `/Kids` reaches is §7.7.3.2's
subject.

#### 3. The report says which door the page came through

`Unsupported::PageDictionary` already said that the page was assembled from part of its dictionary.
It did not say how this reader knew the object was a page, and after this change that is two
different claims: Table 31's `/Type` read off the producer's own bytes, or this reader's inference
from §7.7.3.2 about an object that never declared itself. `DictionaryDamage` carries a
`PageIdentification` and the sentence names the evidence and the entry that carried it. Trap 5's rule
is that a substitution is said out loud; presenting the second as though it were the first is exactly
the silence it exists against.

### What it recovers

All five of `doc/todo/03` §34's witnesses, and nothing else on this disk. The
`standing_count_census` population over `batch1`, `batch2`, `batch3` and `batch6` falls from 12 to 7.

| document | prefix | what it draws |
|---|---|---|
| `GHOSTSCRIPT-699521-0.pdf` | 4, incl. `/Contents` | 795 × 842 with `Hello world` in outlined 30pt Helvetica — the whole page, from a file whose damage is a *second* `/MediaBox` valued at the bare keyword `e` |
| `GHOSTSCRIPT-701846-0.pdf` | 4, incl. `/Annots` | the producer's own 500 × 500, blank, because `/Contents` is among the entries the damage took |
| `GHOSTSCRIPT-698991-0.pdf` | 2, incl. `/Contents` | this reader's sheet, blank, with the `/Contents` object reported unreachable — three sentences said about a 282-byte file |
| `GHOSTSCRIPT-699018-0.pdf` | 1 — `/Annots` alone | this reader's sheet, blank; the single entry is the whole of the evidence |
| `poppler-192-0.pdf` | 2, incl. `/Contents` | this reader's sheet, with `/SH0 is not in /Shading` said by name because the `/Resources` naming it is past the damage |

All five are pinned in `doc/checks/fixed-documents.toml`. **One of them is pinned with a narrower
band than that file's convention and says so**: `GHOSTSCRIPT-699521-0.pdf`'s marks are one 30-point
line on a 795 × 842 sheet and are worth 0.307 of a level, so the seeded ±1.0 would admit a blank page
and pin nothing but the report beside it. A band is only a band when the defect it stands against is
wider than it.

**Six documents stay refused and each for a stated reason** — two whose page objects are not in the
file, one whose `obj` keyword has a regular byte glued to it, one where `/Kids` names such an object,
two whose prefix is zero entries, and `poppler-355-0.pdf`, whose prefix discriminates nothing. Door 2
of §34 — resynchronising past an unreadable value — is **not** taken, and the argument is unchanged:
§7.3.7 states no extent for an entry's value, so skipping to the next `/Name` guesses where the bad
value ended.

### The fixtures, which trap 28 is what asks for

`crates/pdf-model/tests/damaged_page_dictionaries.rs`, and the pairs are the files where the guard
and the rightness condition come apart:

- the same body under a `/Kids` that names the object and one that does not — one character between
  a page and a refusal, which is the door in one line;
- a tree-named prefix whose entries a node may also carry, plus one key in neither table: refused,
  which is §1 above made checkable;
- a tree-named prefix stating `/Type /Pages`: refused, because Table 30 makes that entry the file's
  own statement about what the object is;
- **the guard's own pair** — a tree that *does* reach a page, with a tree-named damaged object
  carrying `/Contents` sitting beside it. The rightness condition holds and the guard does not, so
  the recovery must not run: `/Count` stands and the tree's page is the page.

### What it costs

Nothing for a document whose page tree works. `tree_named` is one descent of the `/Kids` arrays,
under the same two bounds every other walk in the file uses, reached only from `Pages::new`'s
`recovering` arm and beside the pass `scan_for_pages` already makes.

## Part 2 — §14.8.6.3's enclosure `shall`, reported

### Context

Errata Collection 3 (Issues #72 and #719) replaces the subclause's MathML sentence with two:

- the `math` structure element type, as defined in MathML, shall be used to enclose the formula
  under the `Formula` structure element type;
- all MathML structure element types and their attributes shall have the MathML namespace explicitly
  defined.

ADR 0375 read them in the five-hundred-and-fortieth session and declined both, in one step: the
sentence opens on the act of *including* mathematics, so the `shall` is a producer's, and
`CLAUDE.md`'s closed authoring exclusion covers a clause whose requirements fall on a generator. The
row has carried "what stays owed is a validator's report" ever since, in four documents.

### The reading

**Whose `shall` it is has not changed and ADR 0375 was right about it.** What decayed is the step
after: the exclusion says this tree does not *write* such a tagging, and it says nothing whatever
about *reading* one. A file-addressed `shall` is answered here by a report — §7.3.7's row established
it and §14.8.6.2's own file-addressed sentence became one **a single round ago** (ADR 0785), one
subclause away, on reasoning that applies here word for word. This is `doc/habits.md`'s decay shape
where a refusal would have survived the thing that retires it; the reason to record it is that the
refusal was correct when written and stayed on the page for three hundred sessions after the argument
under it moved.

**The first `shall` is reported. The second is declined for a firmer reason than the exclusion**: it
quantifies over MathML's own vocabulary, which ISO 32000-2 states nowhere — §2's normative reference
to MathML Core holds it — so a condition over that list would be this reader's invention wearing the
clause's number.

### The condition, and the two narrow readings inside it

`Tree::mathml_outside_a_formula` counts an element whose type ends at `math` **in the MathML
namespace** with no `Formula` anywhere above it.

- **The namespace is part of the type.** §14.8.6.2 is the clause that makes a name mean what its
  namespace makes it mean, so a `math` ending in some other vocabulary is not the type the sentence
  names. The mirror holds on the other half: a `Formula` in a foreign namespace is not §14.8.4's and
  encloses nothing, which is why the ancestor test goes through `standard_role`.
- **`under` is read as *anywhere under*.** The sentence does not say *immediately*, so an element
  with a `Formula` ancestor satisfies it on either reading and only one with none at all breaks it
  on both. Reporting the intersection is trap 11's rule: fire on what the clause states, not on a
  word it did not write.
- And `Math` is not `math` — NOTE 2 says the type "is all lowercase to match the MathML 3.0
  specification", and §7.3.5 makes a name's identity its bytes.

### Where it goes, and the cheap gate

`viewer_core::notes::about`, once when the document opens, beside §14.8.6.2's. It costs no mark on
any page, so an `Unsupported` would take a page out of the oracle's diagnosed set to say something
that is not about it.

The walk is 151 ms on the largest tagged document in reach, so the elements are read only where the
root's `/Namespaces` array names the MathML namespace — §14.8.6.2: "[i]f the structure element is in
an explicit namespace, then that namespace shall be identified in the structure tree root
dictionary's Namespaces array entry", and §14.8.6.3's second `shall` puts every MathML element in an
explicit one. **The same limit follows and is stated rather than hidden**: a file that also breaks the
`/Namespaces` sentence is not seen, and closing that costs the walk on every tagged document.

### Calibration

Trap 13, both directions, and three plants rather than an assertion that the tests discriminate:

| plant | test that fails |
|---|---|
| the namespace requirement dropped from the type test | `a_math_element_in_another_namespace_is_not_mathml` |
| the ancestor read by name rather than through `standard_role` | `a_formula_in_another_namespace_does_not_enclose_the_mathml` |
| the `enclosed` flag not carried down the descent | `a_mathml_formula_below_a_formulas_descendant_is_not_counted` |

No plant failed a test about a different sentence, and the planted violation passes under all three,
which is what says the six fixtures are about six conditions rather than about one walk. The pair in
`viewer-core` adds §14.8.1's condition — the subclause is inside §14.8, whose requirements are on a
tagged PDF.

### The population

**No witness**: `doc/pdf.js`, the four `doc/corpora/` submodules, this project's fixtures, and the
65 944-document `CC-MAIN-2021-31` crawl, measured by `examples/absence_audit` with the reader that
decides the report rather than with a grep. That is consistent with ADR 0785's finding that four
documents in reach declare a `/Namespaces` array at all and none of them declares MathML.

## Consequences

- `pdf_model::Pages`' recovery has two doors and the report names which one a page came through.
- `doc/todo/03` gains §35; §34's first hand-on is closed and its second is not.
- §14.8.6.3 keeps `implemented` and its row loses the "what stays owed is a validator's report"
  sentence it has carried in four places.
- `viewer_core::notes` is seven clauses rather than six.
- `doc/todo/48`'s three owed items are all closed; what that file still owes is its own steps 4
  and 5.
