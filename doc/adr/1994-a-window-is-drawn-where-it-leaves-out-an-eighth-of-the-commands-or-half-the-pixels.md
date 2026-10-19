# 1994 — A window is drawn where it leaves out an eighth of its page's commands or half its pixels

Session 1579, slot 3 of batch ninety-two, ADR 1980 §4's residue under the project owner's "switch
to code improvements … until I say otherwise" of 2026-10-14. Status: **accepted** and **built** in
`viewer-host` (`oversize::oversized`, `oversize::largest_centred`) and `pdf-render`
(`DisplayList::share_reaching`). Amends ADR 1930's half. Context: ADRs 1868, 1892, 1930, 1941,
1980, 1986. Tests: `reach::tests::the_share_a_window_keeps_of_a_short_list_is_its_selection_counted`
and `…_of_a_long_list_is_estimated_within_a_percent`; `viewer-host/tests/oversized_pages.rs`'s
`a_page_whose_window_is_more_than_half_of_it_and_leaves_out_most_of_its_commands_is_drawn_as_it`.
Drive: step `82-zoomed-window`, its judgement moved to the bound. Instrument: `viewer-host`'s
`examples/window_against_whole`, which now prints the commands each window keeps, the estimate and
its cost, and what the page's own target leaves out. Cites no clause and moves no ledger row.

## 1. What was measured

One `--clock` hold on this machine, headless (its log in the round's scratch, load 6.9–9.1, the
wrapper's nice 19), and no figure from the owner's loop: a toolkit draws its window on the
processor, and the loop is for what needs the owner's display or adapter. HEAD
`58223a35`, three rounds a magnification, 1200 × 900, the page centred and the largest window
centred on it, milliseconds from the ask to the landing, whole page / window. Every digest of a
magnification agreed across its rounds. "Kept" is the share of the page's commands the window keeps.

| page (commands) | area | kept | 4 threads | 24 threads |
|---|---|---|---|---|
| witness (4 408 405) | 83 % | 98.3 % | 629–682 / 660–784 | 450–472 / 466–499 |
| | 70 % | 92.7 % | 745–805 / 643–782 | 455–499 / 460–492 |
| | 63 % | 83.0 % | 727–775 / 622–711 | 460–539 / 426–442 |
| | 49 % | 64.0 % | 780–866 / 551–601 | 483–538 / 359–382 |
| `issue12810.pdf` (38 582) | 68 % | 98.5 % | 136–138 / 137–146 | 102–117 / 103–106 |
| | 61 % | 97.9 % | 108–159 / 152–153 | 115–124 / 114–116 |
| | 48 % | 78.7 % | 125–185 / 138–143 | 140–142 / 104–111 |
| `issue1905.pdf` (2 731) | 88 % | 76.1 % | 162–176 / 158–159 | 97–137 / 100–123 |
| | 68 % | 55.7 % | 262–280 / 210–220 | 190–199 / 188–218 |
| | 61 % | 45.1 % | 320–325 / 232–237 | 211–238 / 226–271 |
| `22060_A1_01_Plans.pdf` (70) | 88 % | 75.7 % | 36–40 / 32–34 | 28–32 / 25–29 |
| | 68 % | 47.1 % | 55–57 / 39–41 | 46–49 / 32–36 |

**The hypothesis held in its quantity and not in its figure.** Area no longer decides:
`issue12810.pdf`'s marks lie in the page's middle, so a window of 61 % of it keeps 97.9 % of them and
ties, where the witness's window of 63 % is ahead by up to 100 ms. Commands kept decide on the two
pages drawn per command — ahead at 83 %, tied at 92.7 % and above — so the bound is seven eighths,
not the four fifths the brief supposed. On `22060_A1_01_Plans.pdf`, seventy commands mostly
images, the pixels decide and every window lands first.

## 2. Decision

1. **A page is drawn as its window where the largest window, centred, is part of it and keeps at
   most seven eighths of its commands or is at most half its area**, or where its whole raster is
   past `viewer_core::MAX_PIXELS` as before. The half stays for a page drawn per pixel, of which a
   page of one image is the case: its one command reaches every window.
2. **The share is estimated, on the thread that asks**: `DisplayList::share_reaching` asks at most
   4 096 commands at an even stride whether they reach the window, as `MarkBounds::misses_target`
   asks with its margin. Within 1.5 points of the exact share on the four pages, in 80–580 µs. The
   exact share is `MarkBounds::measure`'s on every core, and a toolkit thread that asked the pool for
   it while a draw held the pool would wait for that draw.
3. **The centred window is the one priced**, because the question is asked once per request, before
   the view places a window. One placed elsewhere keeps another share, which at worst ties.

## 3. What this does not do

- **`issue1905.pdf` on twenty-four threads**: its windows of 61–88 % keep 45–76 % of its commands
  and tie with the page there, while on four threads they lead by 4–90 ms. Its commands are not of
  equal cost. The bound is drawn by the two pages whose commands are.
- **The confined window keeps its population.** Its worker draws a window only past
  `MAX_PIXELS` for marks that cannot cross (ADR 1986), and the wire admits a window only there. The
  window's shape, the viewport and a quarter of it round it, is unchanged, so the two statements
  ADR 1986 §2.1 names still agree. Taking the bound there is a wire change for a round that owns the
  protocol.
- **The whole page's 20–150 ms of ADR 1980 §3 was not a cull.** The census shows the page's own
  target keeps every one of the witness's 4 408 405 commands at every magnification. `reaching`
  answers `None` there before and after ADR 1980, so the whole page's draw is the same work. The
  instrument draws each round's whole page after the window before it, and before ADR 1980 that
  window copied its kept commands and let them go. Making that copy again before each whole draw
  (`WINDOW_AGAINST_WHOLE_COPY`, two passes each way) made the whole page 0–100 ms later at 1.7×,
  no different at 1.85× and 24–135 ms later at 2.2×. The effect is not monotone in the copy's
  size, so its mechanism (the allocator or the reference counts' cache lines) is not shown here.
