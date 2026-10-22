# 1694 — Direct ICC paint follows current state

Session1694,2026-10-22. ADR2222/2223; §§8.6.5.8,8.6.5.9,11.7.5.3.

**Change.** Retain resolved direct ICC profile and exact operands alongside inline
RGB sources; actual `read_in`, initial/reset and malformed-operand semantics survive.
Existing two pre-alpha slots compare source family/profile/components and complete
current conversion context. Selection seeds them; save/restore retains ownership.
Pattern binding remains first, generic ICC tints convert directly, and cell setup
clears unrelated ordinary sources. Alpha and actual-paint reports stay outside caches.

**Gates.** Independent baseline13:2 passed,11 failed with stale colour.
Candidate13 passed after root corrected intermediate8bit alpha oracle (ADR2220),
without loosening tolerance; cold and inherited group paints agree at all3stages.
Warnings-denied model all-target clippy/fmt and owned/existing focused controls:
22 rendering-intent,2 mixed-paint,10 pattern-tint passed; exit 0,33s,0.57GiB.
Isolated candidate13 passed; source-retention/profile-identity/context omissions
all fail intended assertions with exit 101; unchanged independent tests, exact
source restoration. Driver exit 0,157s hold/170s elapsed,0.65GiB.
Full model+colour:2301 passed,21 skipped,62.615s tests; exit 0,173s hold,
394s elapsed,3.90GiB. Integration accepted with launch limits (1697).

**Cost.** Fixed0eb4d278 before/after exports carry identical instrumentation and
exactly3 production delta paths. OrdinaryICC, genericICCtint, mixed and RGB
populations remain separate; required-current-state work is labeled explicitly.
Release builds exit 0,116/102s,.88/.82GiB.12balanced CPU2pairs/48guards pass;
exit 0,343s hold/506elapsed,.08GiB; source/binary hashes unchanged.
Paired medians repeat1.0471; component/profile restore1.2018/1.2087,
+1.400/1.872ms per20k marks (~70/94ns permark). Root accepts bounded
correctness cost after integration; no performance-neutral claim or cache expansion.
Required intent/BPC work adds2.133/2.401ms; stale-output baseline is separate.
Generic tint ratios.9816–1.0077,mixed.9999–1.0127,RGB1.0111/1.0193.
All samples retained, including repeat ratio range.7236–1.4347; ADR2223 table.

**Limits.** Other colour families retain their previous fallback; no broader cache
unification or clause completion is claimed. Root owns shared ledger and integration.
Scope:/home/AI/paths-1694.txt. Receipts:scratchpad/r1694/.
