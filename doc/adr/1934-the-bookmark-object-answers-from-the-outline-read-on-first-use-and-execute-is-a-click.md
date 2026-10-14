# 1934 — The bookmark object answers from the outline, read on first use, and `execute` is a click

Session 1549, a script round of batch eighty-seven. Status: **accepted** and **built**. Context:
`doc/questions/A193` (the scope rule inside the tiers), RFC 0008 sections 4.2, 4.3 and 6.2, ADRs
1591, 1762, 1904, 1922. Code: `crates/pdf-model/src/view/{script_bookmarks,script_model,scripts,
script_sites}.rs`, `crates/pdf-script/src/{wire,surface}.rs`, `crates/pdf-script/src/engine/
{bookmarks,bridge}.rs`, `crates/viewer-core/src/viewer.rs`. Tests: `crates/pdf-script/tests/{hook,
fixtures,wire}.rs`, `crates/viewer-core/tests/script_triggers.rs`. Clause: §12.3.3.

## 1. What changed, and what it amends

ADR 1922 §2 refused `children`, `color`, `execute`, `name`, `open` and `style` because the realm was
not handed the outline, and named handing it over as the next step. This takes that step. The
`REFUSED` row for the six is gone. The four methods that add, move, remove or re-script an item stay
RFC 0008 section 4.3's Tier 2 rows.

## 2. Decision

- **What crosses.** `DocumentState::outline` carries every item of the outline as a flat list in the
  order a reader meets them, each after its parent and naming its parent by its place. Each item
  carries its object, Table 151's `/Title`, the sign of `/Count`, Table 152's two bits, and `/C`. It
  is read through `pdf_model::outline::Outline::read`, the reading the panel draws, so a script walks
  the tree a person sees. A realm is told at most `MAX_BOOKMARKS` (2^15) items, half the wire's count
  as for the pages. Past it the reading says so once. The wire is version 14.
- **Read on first use, not at the open.** The outline is read once a script of the document spells
  `bookmarkRoot`, or an outline item's own script runs. This is ADR 1762's rule for the pages' words,
  for the same reason: a request is data built before the script runs. A script that builds the name
  at run time meets a refusal saying the outline was not told. It does not meet an empty tree. So
  `open_kinstructions` cannot move. The launch path supplies no runner, and a document with no script
  naming the outline never reads it for its scripts.
- **The root is the outline dictionary.** Table 150 states no title, colour or flags, and its
  `/Count` "cannot be negative". So the root's `name` is the empty string, `open` true, `color`
  Table 151's default black, `style` 0. These are documented choices.
- **`style` is Table 152's bits as a number.** Bit 1 (value 1) is italic and bit 2 (value 2) is bold.
  These are the reference's own numbers for italic, bold and both.
- **`open` is the file's.** §12.3.3 lets a person open and close items in the panel, but the panel
  is the host's, and the realm is not told what a person did there.
- **Every write is refused.** A write to `name`, `open`, `color` or `style` changes what the
  document states, so it gets Tier 2's structure reason. A write to `children` or `parent` is refused
  as read-only.
- **`execute()` is a person's click.** It becomes `ScriptEdit::Bookmark`. The view state holds the
  item only where it is one of the outline the realm was told: a confined worker's answer is data,
  and any other object is refused with a sentence. The viewer then activates it through
  `interact::activate_object`, as a row of the panel is activated (ADR 1904), after the command.
  What an item's own script executes is held for the next command, so a ring of items cannot hold
  one. At most 16 requests are held, and the rest are said. Executing the root does nothing.
- **`event.target` at a `Bookmark` event is the item.** `ScriptSite::Outline` names the item's object
  (tag 7 with its number and generation). This replaces the null of ADR 1904 §2.

## 3. Cost

The outline crosses the wire whole each time the document's state is re-told, as the words do.
The bound is the reason that is acceptable: 2^15 items. `outline.rs` already bounds its own reading
at 2^16 and places the corpus's largest outline two orders of magnitude below that.

**Gates.** Four plants, each applied and reversed as a patch: the reading call removed, the
membership check removed, `execute`'s note removed, and the viewer's carry-out removed. Each turns
at least one of the five new or changed tests red.
