# 766 — The breakdown seven hundred rounds old

2026-08-25. A general-improvement round, subject chosen rather than assigned.
Decision: [ADR 0694](../adr/0694-the-copy-a-cached-font-was-still-paid-for.md).

## What was chosen and why

762's rule, applied to the other half of the pair it came from. ADR 0687 re-took
`callgrind_rasterise`'s composition because the total had been re-taken five times and the
breakdown once, six hundred rounds earlier. `callgrind_interpret` is the mirror: its total is in
sessions 153, 162, 175, 185, 195 and in ADR 0677's table four rounds ago; its composition is one
paragraph in `doc/performance.md` and it is the **fifty-eighth** session's.

The baseline this round took reproduces ADR 0677's total to nine digits — 1 278 427 485 against
1 278 428 629 — which is worth recording because it settles which half of the pair had decayed.

## What moved

`callgrind_interpret`, page 101 of ISO 32000-2, fifty renders, one sitting, two arms built from
one tree:

| | Ir | |
|---|---:|---:|
| before | 1 278 427 485 | |
| `Tf` asks the cache before resolving the font dictionary | 1 247 561 146 | −2.41% |
| a cell per code inside the Adobe Glyph List table | 1 182 345 844 | **−7.52%** total |

The example's own output — the display list's command count — is **150 350** in both arms.

`encoding::text_for` is called 67 200 times before and **8 850** after, which is the number of
codes the page shows. `Interpreter::load_font`, `Document::get` and `String::from_utf8_lossy` are
called 350 times where they were called 14 000, and `BTreeMap::clone_subtree` is not called from
`Interpreter::font` at all.

## Gates

The change is in `pdf-model` and `pdf-font`, so §2 ran whole. Everything unmoved.

One note about the machine rather than the tree: `viewer-host`'s
`a_launch_waits_for_page_one_instead_of_polling_for_it` failed twice under a load average of 35
to 51 from three sibling rounds and passes on its own. It asserts on `Drawing::SETTLE`, a wall
clock, so it is a duration on a shared machine — the thing `doc/todo/02` §2's own bullet warns
about, in a test rather than in a gate.

## The ledger

§9.3.1 and §9.10.2 are the clauses these two functions implement and both rows are `implemented`.
Neither note stopped being true: the change is exact — same function, same arguments, resolved at
a different moment — and no row claims anything about when a code reaches the Adobe Glyph List.

## Load

Load average 2.1 at the start, 14 to 51 through the middle, three sibling rounds beside this one.
Every figure quoted above is a callgrind instruction count, which is why they are quotable at all;
the two gates that spawn reference renderers were held until the machine was quiet.
