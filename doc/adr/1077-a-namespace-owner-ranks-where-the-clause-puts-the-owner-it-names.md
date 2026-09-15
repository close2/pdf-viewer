# ADR 1077 — A namespace owner ranks where the clause puts the owner it names

## Status

Accepted, 2026-09-15. Session 1063, the ledger slot.
Crate: `crates/pdf-model` (`structure.rs`). Check: `cargo test -p pdf-model --lib structure::`.
`§N` is ISO 32000-2 and nothing else; a section of this document is named in words.

## Context

§14.8.5.3 ranks an element's attribute objects into bands before any value is read:

> - The value of the attribute specified in the element's A entry, owned by an owner as specified by
>   the O entry, or, if the value of the O entry is NSO , the NS entry, excluding Layout, PrintField,
>   Table , List and Artifact , if present, and if processing based on the format indicated by the
>   owner value
> - The value of the attribute specified in the element's A entry, owned by Layout, PrintField,
>   Table, List or Artifact , if present
> - The value of the attribute specified in a class map associated with the element's C entry, if
>   there is one

This tree computed none of that. `Tree::attribute` filtered the objects to "PDF-native or `NSO`" and
took the last one in array order, which is §14.7.6's rule for objects *within* a band standing in for
the clause's rule *between* bands. `Owner::Namespace` was admitted unconditionally — neither band 1,
whose condition nothing checked, nor band 2, whose five names it is not. §14.8.5.3's ledger row has
carried that as an open question since session 811 and §14.7.4.2's since session 928, each naming the
other so that neither could settle it alone.

What an `NSO` object's owner *is* comes from §14.7.4.2's closing paragraph:

> When the owner of an attribute object (see Table 360 -Entries common to all attribute object
> dictionaries) is specified by an NS entry, the namespace name shall be considered as identifying
> the owner.  For common namespace names which correspond to the values of owner entries defined in
> Table 376 -Standard structure attribute owners, they shall be considered equivalent.

Session 1024 read that as needing "a list the clause declines to give". Checked against Table 376's
own rows, that is right and narrower than it sounds: the table's sixteen values are names like
`Layout` and `HTML-4.01`, a namespace name is conventionally a URI, and the standard prints no pair.

## Decision

**A namespace owner is put through `Owner::read` and ranked as whatever comes back; a namespace name
the standard gives no owner for is refused by name.** Three cases, and each is derived rather than
chosen except the third.

1. **The name is a Table 376 value.** That is the equivalence, and the whole of the correspondence
   this standard states: the object is a `Layout` object, a `Table` object, and ranks in band 2 — or
   an `HTML-4.01` object, and is refused with the rest of the export formats. Inventing a table of
   URIs mapped onto owner values would be this reader's list wearing the clause's number.
2. **The name is neither that nor a namespace this program processes.** Band 1 by elimination — it is
   none of the five the band excludes — and band 1's condition, "if processing based on the format
   indicated by the owner value", does not hold. So the object is consulted for nothing, and
   `Tree::unranked_owners` names it. §14.8.6.3's `MathML` is the standard's own example.
3. **The name is one of §14.8.6.1's two standard structure namespaces.** Band 1, condition **met**.
   This is the part the clause does not state outright and the reason this is an ADR: §14.8.6.1 says
   the types and attributes of §14.8.4 and §14.8.5 "effectively define a schema for a tagset in PDF"
   and that the two names define it, so the format such an owner indicates is PDF — which is the
   format being processed. The alternative reading, that the standard structure namespace is
   equivalent to the five PDF-native owners and so belongs in band 2, fails on §14.7.4.2's own word:
   the equivalence is with *the values of owner entries*, one value at a time, and this namespace
   corresponds to all five at once, which is not a correspondence.

**The route decides the band as much as the owner does.** Two of the three bands name the element's
`/A` and the third names the class map, so `AttributeObject::from_class` is read beside the owner.
§14.8.5.2's sentence about an export format — "shall be applied only when processing PDF content
based on that format" — is about the *object* and names no route, so a refused owner is refused in
the class map too.

## Consequences

- `Tree::attribute` searches rather than filters: the best band wins between bands, and within one
  band §14.7.6's later-wins rule still does. `Tree::artifact_attribute` gets §14.8.5.8's narrowing
  applied to the *effective* owner, so an `NSO` object equivalent to `Layout` is now excluded from
  Table 385's attributes by the name that clause excludes.
- **On the corpora this changes eight objects and no answer.** 991 documents, 107 tagged, 99 201
  attribute objects: band 1 is empty, 36 elements carry attributes under more than one owner, and
  five state one attribute in two bands — all of them band 2 against band 3, where array order
  already gave the clause's answer. Every namespace-owned object in reach is `MathML`'s, and what
  they state is `lspace`, `rspace`, `mathvariant` and `display`: MathML's vocabulary, which nothing
  here would have asked for. `crates/pdf-model/examples/attribute_owner_census` is the command.
- **No report fires on a refusal**, and that is trap 11 rather than an omission. Declining an export
  format's attributes is obedience to §14.8.5.2, not a gap in this program, and a note saying so
  would fire on five corpus documents to report that the standard was followed. What a caller gets
  instead is `Tree::unranked_owners`, which names what was skipped on an element.
- A round that comes to translate this tree's content *to* one of Table 376's formats inherits band
  1 already built: `AttributeObject::effective_owner` is the one place that decides whether a format
  is being processed.
