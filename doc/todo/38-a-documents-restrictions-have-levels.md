# A document's restrictions are the reader's to set, and they have levels

Status: **the shape is built (ADR 0212, session 373); two of the four levels and any interface for
them are what is left.** No user interface is to be built until the project owner asks for one.
Priority: 38 — capability, and low priority by the owner's own words
Clauses: §7.6.4.2 (Table 22's `/P`), §12.8.2.2 (`/DocMDP`), §12.8.6 and Table 258 (usage rights),
§12.7.5.5 (Table 236's signature field lock — the one restriction addressed to a *named field*
rather than to the document, ADR 0284), §12.7.6.2
Code: `crates/pdf-model/src/restriction.rs`, `crates/viewer-core/src/viewer.rs`,
`crates/viewer-core/src/notes.rs`, `crates/pdf-syntax/src/crypt.rs`

## The policy, in the owner's words

> DRM restrictions are low priority and we should always have the possibility to turn them off. We
> should not implement a UI for them right now, but our finishing product will have a few different
> DRM levels: off, on, ask before operations, warn before operation. I tell this, so that, when we
> encounter them for any reason, they are now implemented in a way, which allows such levels later.

**This is not about the sandbox.** Principle 3's confinement runs the other way — it protects the
reader from the document — and nothing here is negotiable in that direction.

## What the three-hundred-and-seventy-third session built

| clause | what it does now | where |
|---|---|---|
| §12.8.2.2's `/DocMDP` | states which of Table 257's levels withholds which operation | `restriction::asserted` |
| §7.6.4.2's Table 22 | **consulted at last**, for bit 6 and bit 9, with `/R` deciding bit 9 | `restriction::withheld` |
| §12.8.6's `/Perms` | composes the two, because the clause says a permission needs *each* handler | `restriction::asserted` |
| §12.8.2.3's `/UR3` | unchanged, and deliberately: a grant is not a restriction | `view::withdrawn_usage_rights` |

The reading is in `pdf-model` and decides nothing; the policy is one value a host supplies
(`Command::Restrict(RestrictionLevel)`), asked **once per `Edit`**; the refusal leaves as
`Event::Refused { document, operation, notes }`, which carries the operation precisely so that it
can become a question. `viewer-ui` supplies the value with `--ignore-restrictions` and prints the
reason and the way out. The argument, the corpus measurement and the Table 22 revision finding are
in ADR 0212.

## What is left

- **The two levels themselves.** *Ask* and *warn* are `Event::Refused` plus a host that answers
  with a `Command` — the `Event::PasswordRequired` shape — and they are not shipped because a
  variant nothing produces and nothing answers is a level that silently behaves like another one.
  Nothing here is `#[non_exhaustive]`, so adding them fails every consumer's compile until it says
  what it does, which is what makes waiting safe rather than lazy.
- **A user interface**, when the owner asks for one. A menu with four entries and, probably, the
  per-document override the viewer-wide value does not express today.
- **Table 22's bit 5, and the copy operation nothing here can name.** The bit is "[c]opy or
  otherwise extract text and graphics from the document", and this crate hands a host a *readback*
  — the same `Query::Selection` that a drag asks sixty times a second in order to draw a
  highlight. Refusing that would refuse the highlight. The bit also carves itself: "for the limited
  purpose of providing this content to assistive technology, a PDF reader should behave as if this
  bit was set to 1", so §14.9's tree must never be gated by it. What is needed is a host saying
  *this is a copy* — plausibly `Query::LogicalSelection`, whose own doc comment already says "[a]
  host asks this when a person presses copy", made to answer differently under the policy. It
  wants an `Answer` variant rather than `Answer::None`, because a copy that came back empty would
  be a lie about the selection.
- **Annex O's `ef`, which is the same four levels arriving from `doc/todo/39`.** "[S]ecurity should
  be strongly considered when opening an embedded file … a PDF processor may choose to prompt the
  user or even prevent opening of the file" — a *prompt*, which is exactly the ask level, over an
  operation (`Command::Extract`) that no document restricts today. It is the second consumer the
  levels are waiting for.
- **Printing and assembling** (Table 22 bits 3, 11 and 12) will need rows here the day this program
  can print or rearrange pages. Until then they are a capability rather than a permission, and
  `restriction::Operation` deliberately has no arm for them.

## What not to do

- **No user interface**, by the owner's instruction, until it is asked for.
- **No level enum shipped with one caller**, which is why two of four are absent rather than
  stubbed. ADR 0178's lesson.
- **No weakening of what is written.** §7.5.6's incremental update, `Document`'s immutability and
  the signature-withdrawal rule are correctness, not policy: a save that exceeds a usage-rights
  grant must still remove the `/UR3`, because §12.8.6 makes the signature a claim about the file
  and leaving it would make the file lie. Turning the *restriction* off is the reader's; making
  the file assert something untrue is not. The types keep the two apart —
  `RestrictionLevel` reaches `Viewer::refusal` and nothing else, and `withdrawn_usage_rights` is
  reached from `ViewState::save` with no policy in scope at all — and it must stay that way.
- **§12.7.6.2's submit is still not one of these**, re-checked in the three-hundred-and-seventy-third
  session: it is refused because it needs a network this program does not have (principle 3), which
  is a capability rather than a permission, and no level would turn it on.
