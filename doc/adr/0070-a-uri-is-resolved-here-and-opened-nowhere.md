# ADR 0070 — A URI is resolved here and opened nowhere

Status: accepted, 2026-07-31.

## Context

Two of §12.6's twenty action types were `silent` rows and were the largest actions in the corpus
that nobody had written: `URI` at **217 actions in 8 documents**, and `Named` at **9 actions in 3**.
Both were refused by name, and both refusals stated a reason this session found to be only half
true.

`action.rs` said of the first: "URI: a network resource, which this reader has no network to
fetch." That describes the *last* step. §12.6.4.8 says a URI action "causes a URI to be resolved",
and resolving is three things:

1. decide what the URI is — Table 210's `/URI`, against Table 211's `/Base` where the catalog
   states one, by the algorithm RFC 3986 defines;
2. apply Table 210's `/IsMap`, which appends the cursor's position to it;
3. fetch or open whatever the result names.

Only the third needs a network, and the first two are exactly the kind of work this crate exists
to do. The corpus says so too: `issue14802.pdf` states `/URI << /Base (http://example.com/) >>`
and a URI action of `(./relative_link.txt)`, so the one document that writes a base is a document
whose link cannot be followed without step 1.

The second refusal said "Named: a viewer command such as NextPage, which is viewer-ui work". Half
true again: *which* page NextPage reaches is the viewer's, and Table 215's four names and what each
means are the document's.

## Decision

**Read and resolve both, and let the caller do the part that is about this machine.**

`ViewState::perform` now answers a `view::Request` rather than an `Option<Destination>`:
`Display` for §12.6.4.2's destination, `Page` for §12.6.4.12's command, `Resolve` for
§12.6.4.8's URI. The three are exactly the actions that change something a `ViewState` does not
hold — which page is on screen, and what is outside the document altogether — and the split is
the same one that put optional content and hidden annotations in this type in the first place
(ADR 0065).

`crates/pdf-model/src/uri.rs` is RFC 3986 section 5.2's reference transformation and section 5.3's
recomposition. It holds no PDF at all, which is the same shape as `accessibility.rs`.

`viewer-ui` performs the page commands and **prints** the URI. Opening it is not implemented and
is not owed by the clause: handing a string a document controls to a browser is a decision about
this machine, and a viewer that does it silently is a drive-by navigation waiting to happen.

## Why the algorithm is written out

The obvious shortcut is to concatenate `/Base` and `/URI`. It is right for the one shape the
corpus writes and wrong for four of RFC 3986 section 5.4's twenty-four examples: a reference
beginning `/` replaces the base's whole path, one beginning `//` replaces its authority, an empty
one means the base itself, and `..` segments have to be removed *after* merging. Those twenty-four
examples are normative and are this module's tests — the same reason §12.4.2's own worked example
is `page_label.rs`'s test, and this project's habit that "the standard sometimes states answers
rather than rules, and those are the tests to write" applied to a document ISO defers to.

Where the standard defers, the deferral is a citation. §9.7.5.3 hands a `CMap`'s syntax to Adobe
Technical Note #5014; §12.6.4.8 hands a URI to RFC 3986. Neither deferral makes the requirement
optional.

## What is deliberately not done

- **Opening the URI.** Above.
- **Normalising it.** RFC 3986 section 6's normalisation, percent-decoding, and validity checking
  are not applied. A URI a document states is handed on as the document wrote it, because a reader
  that "corrects" a URI has changed where a link goes. This is why an *absolute* reference does not
  go through the resolution algorithm at all: the only thing that algorithm would still do to it is
  remove its dot segments, which is normalisation.
- **Guessing a scheme.** `pr19449.pdf` writes `/URI (foo.bar.com)` and states no base. RFC 3986
  makes that a relative reference, and §12.6.4.8 says such a URI is "interpreted relative to the
  location of the document itself" — a fact about where the file was opened from, which this crate
  does not know. `Uri::relative` says so and the string is unchanged. pdf.js prepends `http://`;
  that is a guess about what a producer meant, and principle 5 forbids matching it.

## The gate this found

`§` in this tree has always meant "a clause of ISO 32000-2", and nothing enforced it. The first
draft of this session's comments wrote `RFC 3986 §5.2`, which is correct writing about the RFC —
and ISO 32000-2 *has* a §5.2, so the citation checker read it, found the clause, and said nothing.
Four spellings of the same mistake were in the tree and one of them was invisible.

`citation.rs` now reports a `§` whose line names another document before it — an acronym and a
number, `RFC 3986 §5.2`, `ISO 15076-1 §6` — as a foreign citation, and the citation gate fails on
one with a message naming the form to use instead. ISO 32000-2 before its own clause number is
deliberately not foreign, because that is the convention principle 5 asks for.

## Consequences

- `silent` falls 174 → 172; §12.6.4.12 is `implemented` and §12.6.4.8 is `partial`, its note
  naming the one step that is not taken.
- `Link` gains `rect`, because §12.6.4.8 measures `/IsMap`'s coordinates from the annotation's
  rectangle even where `/QuadPoints` narrowed the activation region — two different rectangles,
  one clause apart.
- No gate moves: no corpus document states `/IsMap`, and a URI action changes no pixel.
