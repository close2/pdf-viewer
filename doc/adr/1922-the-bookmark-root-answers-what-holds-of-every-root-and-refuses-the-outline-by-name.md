# 1922 — The bookmark root answers what holds of every root, and refuses the outline by name

Session 1543, a docs round of batch eighty-six. Status: **accepted** and **built**. Context:
`doc/questions/A193` (the scope rule inside the tiers), RFC 0008 sections 4.2 and 4.3, ADRs 1591,
1724, 1904. Code: `crates/pdf-script/src/surface.rs`, `crates/pdf-script/src/engine/bridge.rs`.
Test: `crates/pdf-script/tests/fixtures.rs`.

## 1. What was wrong

Adobe's reference, at the pinned commit, defines `this.bookmarkRoot` on its "Doc properties" page,
and the `Bookmark` object on its "Bookmark properties" and "Bookmark methods" pages: seven
properties (`children`, `color`, `doc`, `name`, `open`, `parent`, `style`) and five methods
(`createChild`, `execute`, `insertChild`, `remove`, `setAction`). This realm carried none of it, so
`this.bookmarkRoot` was `undefined` and the reference's own walk, `bkm.children`, failed with a
`TypeError`. RFC 0008 section 4.3 says a call the realm does not perform fails as a
`NotAllowedError` naming it, never as `undefined`. `A193`'s rule says the same of every member
inside the tiers: either built, or refused by name with its reason. ADR 1904 left this to be done.

## 2. Decision

`this.bookmarkRoot` is an object, `Holder::Bookmark`, spelled `Bookmark.` in a refusal.

- **Carried**: what is true of every root without reading the outline. Its `parent` is null, which
  is what the reference says of the root bookmark, and its `doc` is the document, the global object.
- **Refused as Tier 2**: `createChild`, `insertChild`, `remove` and `setAction`. Each changes the
  document's structure, which is RFC 0008 section 4.3's row and the reason `setAction` is refused
  on a field and a layer.
- **Refused with a reason of this program's** (`surface::REFUSED`): `children`, `color`, `execute`,
  `name`, `open` and `style`. Each reads or performs an item of §12.3.3's outline, and the realm is
  not handed the outline. A write to one of them would also change the document's structure.

**What moves a member out of `REFUSED`** is the outline crossing the wire: the view state handing
the realm the items' titles, flags, colours, open state and actions, as it already hands over pages
and annotations. Then the reads become carried members. `execute` becomes a request for the item's
action, which the host performs as it performs an activation (ADR 1904). Building that is a change to
`pdf-model`'s script model, the wire and the worker. It is the next step of `A193`'s rule here, and
this decision does not price it.

**`event.target` at a `Bookmark` event stays null** (ADR 1904). The target is the item, and the
item is exactly what the realm is not handed.

**Gates.** `every_refused_member_throws_not_allowed_error_by_name` reaches the ten listed members
through `this.bookmarkRoot`. `the_bookmark_root_answers_its_parent_and_doc_and_refuses_the_outline_by_name`
runs the reference's own walk. Both fail with the root's installation removed, and pass with it.
