# 0838 — A parent-tree value is an array whether the tree holds it or names it

Session 898. Status: **accepted**. The defect ADR 0839's gate found on its first run, and the
first thing this suite has written that another program could see was wrong and this tree could
not.

## Context

ADR 0834 carries §14.7's structure tree through `split`, `merge` and `pages`. §14.7.5.4 gives the
parent tree two kinds of value, and the carry has to map both:

> For a content stream containing marked-content sequences that are content items, the value shall
> be an array of indirect references to the sequences' parent structure elements.

> For an object identified as a content item by means of an object reference (see 14.7.5.3, "PDF
> objects as content items"), the value shall be an indirect reference to the parent structure
> element.

The array's *members* must not be resolved: an `/MCID` is "a zero-based index into the array", the
content stream crosses byte for byte, and what the carry maps is each member's **identity** — the
source element it names, to the output element that element became. Resolving a member throws that
identity away, which is why `Carry::source_entry` looks the key up with
`pdf_syntax::tree::lookup_unresolved` and why its doc comment said so.

It said so about the wrong object. `lookup_unresolved` leaves the *value* unresolved too, and
§7.3.10 lets that value be an indirect reference to the array — in one sentence that is the whole
of this defect:

> Except where documented to the contrary, any object value may be a direct or an indirect
> reference; the semantics are equivalent.

So for a source that writes `/Nums [0 13 0 R]` with `13 0 obj [12 0 R 15 0 R]`, the match arm that
handles an array never fired. The `other` arm did, wrapping the *reference to the array* in a
one-long array and asking `element_reference` which output element object 13 became — and object
13 is not a structure element, so the answer was §7.3.9's null.

**Every such document therefore got `/Nums [0 [null]]`** where its source had an array of the page's
elements: a parent tree naming nothing, on a page whose content stream still states every `/MCID` it
did before. Nothing in this tree could see it. §14.7 is invisible on the page, so all four of RFC
0002 §9's layers pass — that is ADR 0835 §5's whole point — and `check_structure`'s first property
asks only that the value **be** an array, which `[null]` is.

Of the corpus's tagged documents, **70 of the 83 whose parent tree states a root-level `/Nums`
write the value out of line**, which is the ordinary form rather than an unusual one.

## Decision

**A page key's array is resolved; its members are not.** One line in `Carry::parent_tree`:

```rust
let array = host
    .source(*at)
    .map_or(Object::Null, |document| document.resolve(&value));
```

`Document::resolve` follows the reference chain to one object and returns it whole, so the members
arrive as the references this module maps. A value that resolves to anything but an array keeps
the previous behaviour — a page key whose value is a single element reference is wrapped, which
§14.7.5.3's single-element form is about — and an object key is untouched, because its value *is*
the reference.

`source_entry`'s doc comment now says which of the two objects is left unresolved and why, since
saying "the values are references" of both is what made the arm look right.

## Consequences

- `crates/pdf-transform/tests/merge.rs::a_parent_tree_value_stated_out_of_line_is_carried_as_the_array_it_names`
  is the regression, over a fixture built in §7.3.10's other form. `tagged_document` gained a
  `ParentTreeValue` parameter so that both forms are fixtures rather than one; the direct form was
  the only one any test had, which is why the defect survived the round that wrote the carry.
- The gate that found it is ADR 0839's, and it found it through `mutool show` — mupdf resolving our
  parent tree and printing what it got. **The instrument mattered more than the reading**: this
  round read §14.7.5.4 before building the walk and did not see it, because the clause says nothing
  about §7.3.10 and does not have to.
- ADR 0834's design statement is unchanged and is now true: "a page's value is the source's array at
  its own length and in its own order".
