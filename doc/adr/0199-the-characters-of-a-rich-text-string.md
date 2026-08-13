# ADR 0199 — The characters of a rich text string, and none of its formatting

Status: accepted, 2026-08-06 (session 342).

## Context

`doc/todo/01`'s third sweep — a ledger note whose reason names a **capability** — was run over all
823 rows after three rounds that added verbs. Twenty-four rows matched and twenty-three name a
boundary this tree keeps on purpose. The twenty-fourth was §12.5.6.2:

> What is owed is … the rest of the interactive one: /Subj, /RC, /IRT, /RT and /IT reach a comments
> pane this program has no panel for.

`/RC` is not like the other four. Table 172:

> A rich text string (see Adobe XML Architecture, XML Forms Architecture (XFA) Specification,
> version 3.3 ) that **shall be displayed in the popup window when the annotation is opened**.

A `shall`, about the popup window — and this program has had one since the
three-hundred-and-twelfth session (ADR 0191). The row's reason expired thirty sessions before the
sweep found it, which is this shape's median. `/Subj`, `/IRT`, `/RT` and `/IT` genuinely reach a
comments pane and stay owed; `/RC` names the window that exists.

**Two of those four were not what this ADR said, and the four-hundred-and-eightieth session found
it** (ADR 0315). `/IRT` and `/RT` are a reply relationship *and* a group: §12.5.6.2 gives a group
nine shared entries, two of which — `/C` and `/Contents` — are ink rather than a pane. So this
paragraph made the same mistake one row further down that the sweep above was built to catch, and
it made it in an ADR, where nothing checks. `/Subj` and `/IT` stand.

## Decision

### The characters, and none of the formatting

`popup::rich_text` parses `/RC` with the `xmlparser` §14.3.2's reader already uses and takes the
**element content**. Nothing interprets a `<span>`'s style, colour, size or face.

The reason is `CLAUDE.md`'s own exclusion list read carefully. **XFA is excluded, and the exclusion
is about the architecture rather than about characters.** Rendering XFA rich text needs a
specification this project does not have and has decided not to acquire; extracting the text needs
nothing but an XML tokenizer, and the text is what the clause requires displayed. The clause says
so itself from the other end, in NOTE 1:

> The RC entry performs a similar role to the Contents entry except that the content's textual
> representation is formatted. When both Contents and RC entries are present, it is expected that
> the contents of both entries are textually equivalent.

If the two are textually equivalent, then the text is the thing, and the formatting is what this
processor declines. That is a **documented departure**, not a silence: it is in §12.5.6.2's ledger
row with the reason beside it.

### `/Contents` outranks `/RC`

Where both are present the plain string is used, on NOTE 1's own reading: they say the same thing,
and the one that needs no markup parsed is the one to trust. `/RC` is read only where there is no
`/Contents` — which is exactly where a window would otherwise open empty.

### A paragraph is a break

§12.5.6.2 states the rule for the plain form — "[w]hen separating text into paragraphs, a CARRIAGE
RETURN (0Dh) shall be used and not, for example, a LINE FEED character (0Ah)" — and the rich form
spells a paragraph as an element. So a closing `</p>` and a `<br/>` become the newline the plain
form would have carried, and nothing else in the markup changes the text.

### Malformed markup keeps what it read

The tokenizer stops at the first error and the text before it is kept. A popup is a person's
comment; half of one is better than none, and a producer's stray ampersand may not take a window
away. The contrast with `xmp.rs` is deliberate and is the difference in what a failure costs: a
metadata packet that will not parse is *reported* as unread because a caller can act on that, while
a popup has no second reader to tell.

Two budgets bound it: 64 KiB of markup (`xmp.rs`'s packet budget is 8 MiB; a popup is a paragraph
or two) and 4096 paragraph breaks, which bounds the one place the walk appends without consuming
input.

## Consequences

- **The corpus cannot exercise this, and that is measured rather than assumed.**
  `examples/markup_text_census`, new here, walks every page of all 974 documents:

  | | |
  |---|---|
  | annotations stating `/Contents`, `/RC` or a `/Popup` | 259 |
  | stating a `/Popup` | 115 |
  | stating `/Contents` | 197 |
  | stating Table 172's `/RC` | 71, over 33 documents |
  | **stating `/RC` and no `/Contents`** | **18** |
  | popup windows displayed, with text in them | 115, 66 |

  All 18 are on `issue13447.pdf`, and that document states **no `/Popup` at all** — so not one
  corpus popup changes, before or after. The census was run with the fallback in and out and
  printed 66 both times.

- **So this is trap 8's converse and it is shipped on purpose.** A corpus finds what documents
  contain, not what the standard says; a `shall` with no witness here is still a `shall`, and
  §12.5.6.2's row now carries the number instead of an assumption. The module's own three tests are
  the only thing defending it, which is the same position §12.7.4.3's comb, right-quadded and
  password fixtures are in.

- **What stays owed** is the other half of the same sentence: `/Subj`, `/IRT`, `/RT` and `/IT`,
  and an `/RC` on an annotation that names no popup window to display it in. Those need a comments
  pane, which is a panel this host does not draw. **`/IRT` and `/RT` came off that list in the
  four-hundred-and-eightieth session** and only their reply half is still a panel's; ADR 0315 has
  §12.5.6.2's group and the nine entries it shares.

- `xmp::unescape` became `pub(crate)`. XML's five predefined entities and its numeric references
  are the format's, not the packet's, and two copies of that table would be two tables that drift.
