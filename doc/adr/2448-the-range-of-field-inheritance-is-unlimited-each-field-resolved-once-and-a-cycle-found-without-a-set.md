# 2448 — The range of field inheritance is unlimited: each field is resolved once per document, and a cycle is found without a set

Session 1807, a "clause" round of batch one hundred and twenty-one, 2026-10-27.
Status: implemented — §12.7.4.1's row leaves `departed` for `implemented`.
Context: ADR 1198 (the bound and its argument), ADR 2439 (the reading that found its reason half
false), `crates/pdf-model/src/inheritance.rs`, `crates/pdf-syntax/src/document.rs`
(`Document::derived`), `crates/pdf-model/examples/field_walk_cost.rs`.
Clauses: ISO 32000-2 §12.7.4.1 (Tables 226, 227), §12.7.4.3 (Table 228), §12.7.5.2.3 (Table 230),
§12.7.5.3 (Table 229), §7.3.9.

## 1. The two halves of the bound, and what ends each

> An interactive PDF processor shall not limit the range of inheritance for field dictionaries.

ADR 1198 kept a bound of 256 `/Parent` links for two reasons, and ADR 2439 separated them. **A cycle**
was priced at "a set per field", and a cycle needs no set: `inheritance::Ancestors` walks the chain
with Brent's method — one remembered position, moved to the walk's own position each time the walk
has gone a power of two steps past it — so a chain that ends is walked once and a chain that loops
is stopped within twice its loop's length of the remembered position entering the loop. Only an
object read by reference is compared, because a dictionary written inside its child cannot contain
itself. **A deep acyclic chain walked once per widget** costs widgets × depth, and that half is what
ADR 2439 left standing. It ends because no widget walks the chain any more: `inheritance::Lineages`
keeps every field's resolved entries beside the document, keyed by the field's object (§12.7.4.1:
each field "shall be an indirect object"), filled top-down the first time a widget below it is read.
The cost of a hierarchy is one merge per field dictionary. Neither half needs a number of links.

## 2. Where the memo lives

`Field::read` is called from some thirty places in `pdf-model`, most of which hold only a
`&Document`. A memo in `ViewState` would have widened each of them and still missed the callers
without a view. **`pdf_syntax::Document::derived::<T>()`** is a memo slot per type, made empty on
first use: the syntax layer holds it without naming what is in it, so no layer boundary moves, and
a memo keyed by its own type cannot collide with another crate's. The memo is responsible for being
a function of the bytes, which `Lineages` is — what a person entered, a reset or an import is applied
by `Field::read` over what the memo holds — so `CLAUDE.md`'s "`pdf_syntax::Document` stays
immutable" is unchanged in substance: the document still answers every question from its bytes.

## 3. What a field resolves, and the one reading that changed

`READ_UP_THE_CHAIN` names thirteen entries: the eight Tables 226, 228, 229 and 230 mark
inheritable (`/FT`, `/Ff`, `/V`, `/DV`, `/DA`, `/Q`, `/MaxLen`, `/Opt`) and five this crate's readers
already took from the nearest dictionary stating them (`/TI`, `/I`, `/Kids`, `/RV`, `/DS`), because
a widget not merged with its field states none of them. `/AA` is resolved by trigger, because a
widget's `/AA` and its field's hold different triggers (Tables 194 and 199). A reader looks through
two dictionaries: the widget's own, then the resolved one.

**Each entry is the nearest *stated* value**: present and not null, since §7.3.9 makes "the null
object as the value of a dictionary entry" "equivalent to omitting the entry entirely". Before, each reader carried its own test while walking —
`/Opt` was "the nearest array" for the appearance and "the nearest non-null" for a script's
`change_ex`, `/MaxLen` "the nearest integer" for one and "the nearest non-null" for another — so one
field could show two readers two values. Now a wrongly typed value on an ancestor is that field's
statement, which the reader asking judges, and a farther ancestor's value of the right type is no
longer reached past it. Only a damaged file can tell the two readings apart; section 5 says
whether any page moved.

## 4. Down the tree, and what stays bounded

The two walks down `/Kids` (`view::field_tree`'s name table and `widgets_under`) take a stack of their
own instead of recursing, so the bound that guarded the thread's stack goes too; each keeps the set of
fields it entered, because a field reached twice going *down* may be reached by two paths and not only
round a loop. A `/Parent` chain that is a cycle is reported where a field is read — the appearance
refuses with "its field's /Parent chain returns to a field it has already passed", and an export names
it in `Submission::owed` — and a qualified name built up such a chain is none.

Not this clause's: `pdf-transform`'s merge and archive walks (256) and `pdf-signature`'s revision walk
(64) are a writer's and a verifier's, not an interactive processor's inheritance, and keep their
bounds and their ADRs (1198, 2154). The three walks a save takes through its own update
(`view::writing`) find a cycle the same way and are not memoised, because what they read is the update
and not the bytes.

## 5. What it costs on the corpus

`examples/field_walk_cost` over the tracked 974 reads 103 form documents and 782 fields, and times
`form::fields` alone, interleaved three times a side in one `--clock --nice 0` hold, one-minute load
2.3 and no process holding the render node. HEAD reads them in 0.008, 0.008 and 0.009 s, this tree
in 0.009, 0.008 and 0.008 s: no slower at the instrument's millisecond. A first cut, which copied every
field's resolved dictionary whether or not the field stated anything, read them in 0.010, 0.010 and
0.009 s against 0.008, 0.008 and 0.007 s — a real file's chain is one to four links, so the walk it
replaced was cheap and the memo's own overhead showed. What closed it is that a field stating none of
the entries shares its parent's resolution by reference count (`Resolved`'s two `Arc`s), and a widget
with no `/Parent` takes one shared empty resolution. The corpus's figure says nothing about the hostile
case the memo exists for; the four-thousand-link test is that case's witness, a plant of a 512-link
bound fails it and `tests/variable_text.rs`'s deep chain alike, and the corpus's own six arms are
unchanged by page.

## 6. Consequences

The tests: `inheritance.rs` holds a cycle of one, of two and a tail into a loop of three each ending
as a cycle, and a chain of four thousand links with a widget at each reading its top's `/V`;
`tests/variable_text.rs` draws a value 4 096 links up and refuses a cycle by name, each the other's
control (trap 8). `examples/field_walk_cost` is the instrument for the cost.
