# 1701 — Qt frames use one local image sink

Session 1701, 2026-10-22. ADR 2236/2237; no clause/status change.

**Change.** One refresh-local placement snapshot supplies ordered borrowed pixels
and descriptors to a synchronous C++ image sink. It copies each accepted raster
once and retains no Rust slice or host/window pointer. Rust borrows end before
widget updates and `painted`. No persistent cache or extra unsafe operation.
Transition, invalid and empty handling preserve their existing policy.

**Bridge.** Isolated pinned-CXX vector and shared-wrapper return attempts compile
C++ but fail Rust lifetime shims; exit 101, 6/1 s holds, .36/.03 GiB. Two synchronous
sink controls compile and execute both languages, exit 0, 1 s/.03 GiB each.
The rejected forms and exact source remain outside the shipping tree.

**Gates.** Scoped fmt, warnings-denied all-target clippy and nextest pass, exit 0:
43 tests passed, 1 skipped, .182 s tests, 30 s hold/.69 GiB, 190 s including queue.
Earlier test-only lint failures were corrected. Byte/pointer/geometry/order,
transition, short-raster, empty and stateful spread zoom controls pass. A private
isolated double-query mutation fails the intended 2 != 1 assertion; control driver
exit 0, 162 s hold/1.91 GiB. Its target never replaces shared test artifacts.

**Display.** Immutable candidate CPU/Xvfb drive passes all five scenarios: zoom
stand-in, retained page, oversized zoomed window, transitions and page-as-built;
5 works, 0 wrong, exit 0, 33 s hold/1.32 GiB, 723 s including queue. Owned Qt
production hashes agree across the release build (288 s hold/1.91 GiB).

**Cost.** Isolated release build passes, 348 s hold/1.97 GiB. Twelve balanced
pairs per one/two-frame layout preserve copied images and rectangles; query counts
fall from 3/5 to 1. Both quiet guards pass; exit 0, 62 s hold/.08 GiB, 206 s elapsed.
Retrieval medians fall 149.70→61.51 ns and 344.41→83.81 ns. Full-copy/delivery paired
ratios .9435/.9982 have ranges .733–1.364/.929–1.103: no reliable overall gain claimed.
A missing optional script-worker preparation stopped the first queued clock before
grant, with zero samples; the scripts-off fixture records its required worker.

**Limits.** Retrieval/copy/collection/host-followup and widget-paint boundaries
remain distinct. Neither CPU display checks nor fewer queries prove the owner's
Qt lag cause or compositor presentation. GTK blur/deep drift remain separate.
Root owns combined corpus/integration acceptance. Full round duration is not
inferred from scoped holds; combined integration remains pending.
