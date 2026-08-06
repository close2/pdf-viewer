# A document's restrictions are the reader's to set, and they have levels

Status: **policy stated by the project owner in the three-hundred-and-fifty-eighth session and
written into `CLAUDE.md`; the code does not have the shape yet.** No user interface is to be built
now — what is owed is that the existing refusals stop being hard-coded, so the levels can be added
without revisiting them.
Priority: 38 — capability, and low priority by the owner's own words
Clauses: §7.6.4.2 (Table 22's `/P`), §12.8.2.2 (`/DocMDP`), §12.8.6 and Table 258 (usage rights),
§12.7.6.2
Code: `crates/pdf-model/src/view.rs`, `crates/pdf-model/src/signature.rs`,
`crates/viewer-core/src/notes.rs`

## The policy, in the owner's words

> DRM restrictions are low priority and we should always have the possibility to turn them off. We
> should not implement a UI for them right now, but our finishing product will have a few different
> DRM levels: off, on, ask before operations, warn before operation. I tell this, so that, when we
> encounter them for any reason, they are now implemented in a way, which allows such levels later.

Four levels — `off`, `on`, *ask*, *warn* — and the binding part today is the **shape**: a
restriction is written so that the policy is asked once, in a place a host can supply, rather than
hard-coded as a refusal at the point of the operation.

**This is not about the sandbox.** Principle 3's confinement runs the other way — it protects the
reader from the document — and nothing here is negotiable in that direction.

## Where the tree already refuses, and how

Three places, all currently hard-coded to *on*:

| clause | what it stops | where |
|---|---|---|
| §12.8.2.2's `/DocMDP` `/P 1` | a person typing into a field | `view::permits_form_filling`, called at the top of `ViewState::set_field` |
| §12.8.6 / Table 258's `/UR3` | nothing — but a save beyond the grant **withdraws the signature** | `ViewState::save`, through `UsageRights::grants` |
| Table 22's `/P` | nothing yet | `Document::permissions` carries the flags and no operation consults them |

Two of the three are the shape to change. The third is already right in one respect and worth
saying: Table 22's flags are *carried* rather than acted on, so the day something consults them is
the day the policy has to exist — and that is this file.

**§12.7.6.2's submit is not one of these.** It is refused because it needs a network this program
does not have (principle 3), which is a capability rather than a permission, and no level would
turn it on.

## What the shape has to become

- **One policy value, supplied by the host.** `viewer-core` is where a host reaches, so the level
  belongs on the viewer — set at open, or per document. `pdf-model` may not decide it: rule 2 of §0
  says the host supplies what the core cannot know, and how much a person's own program obeys
  somebody else's file is exactly that.
- **Asked once per operation, not once per widget.** `set_field` refuses per widget today; an
  *ask* level needs one question per thing a person did, which is one per `Edit::SetField`.
- **A refusal that can become a question.** The `on` level answers "no", `off` answers "yes",
  and *ask* and *warn* both need the operation to be **describable** before it happens — which
  means the check produces a *reason* rather than a boolean. `viewer_core::notes` already words
  such reasons for a person when a document opens, which is the vocabulary to reuse.
- **The event, not a callback.** `Command`/`Event` has no request-reply, and adding one for this
  would be a vocabulary change. The shape that fits what is already there is the one
  `Event::PasswordRequired` uses: the core emits *what it is about to refuse*, the host answers
  with a `Command`, and the edit log records what was done rather than what was asked (ADR 0196's
  rule).

## What not to do

- **No user interface**, by the owner's instruction.
- **No level enum shipped with one caller.** ADR 0178's lesson is that a model entry with no
  consumer is a row that goes stale; this waits for the round that has a second restriction to
  route through it, or for a host that wants to ask.
- **No weakening of what is written.** §7.5.6's incremental update, `Document`'s immutability and
  the signature-withdrawal rule are correctness, not policy: a save that exceeds a usage-rights
  grant must still remove the `/UR3`, because §12.8.6 makes the signature a claim about the file
  and leaving it would make the file lie. Turning the *restriction* off is the reader's; making
  the file assert something untrue is not.
