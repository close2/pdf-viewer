# Q828 — Which of the gates "build now" cannot lift does the owner release?

Source: round 1994 (ADRs 2799, 2800), the ledger lane, auditing `doc/questions/A800` item 2.

## What stands

`doc/todo/76` lists every gate the tree's documents state. A800's "build now" lifts the effort-only
ones, and the first of them is built (ADR 2799). The rest of this list is not effort: each needs
something only you can give.

1. **Purchases.** ISO/IEC 11172 and 13818-3, for MPEG-1 and MP3 clip timing (ADR 2489); CIE 131,
   for JPX's `CIEJab` beyond its fallback (A348); T.801's 2004 edition (A169); ISO 19444-1 for
   XFDF (A97, only if the edition difference ever bites).
2. **The launcher** (A270, A595): two Vulkan variables wait "until a launcher exists", and the
   tree ships none. Whether it ships one is a packaging decision.
3. **Signing the Windows zip** (A636): a code-signing identity you pay for and hold, after a person
   has run the window on a Windows desktop.
4. **The scope exclusions' reopening triggers** (`doc/scope-ratifications.md`): Tier C (decoding
   audio and video, "until the owner decides to build the player"), Tier D (U3D and PRC), XFA's
   dynamic half, and overlay and underlay (A254 question 7). `CLAUDE.md` says an exclusion is
   revisited "by argument, never by attrition", so A800 is not read as lifting them.

## The question

Does A800's "build now" reach any of these four groups? If it does, which?

## Recommendation

- **1: no purchase now** except where a ledger row's final state waits on the text alone. None of
  these does: no document in reach states MPEG-1 or MP3 timing, `CIEJab`, or an XFDF feature the held
  3.0 specification lacks.
- **2 and 3: no change.** Both are about shipping, not about what a document shows.
- **4: keep the exclusions, and take XFA's measurement now.** Pricing page one of the corpora's
  `/NeedsRendering` documents is effort, and it is what the exclusion's own trigger asks for
  first; it is `doc/todo/76` row 10 and needs no answer to run.

## What the tree does meanwhile

Every gate in this list stays as written, and `doc/todo/76` names it with its kind. The effort-only
gates are built in that file's order.
