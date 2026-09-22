# 1200 — The frontier map re-derived from the live ledger, and two sentences that outlived their code

Instruments and consolidation round of batch thirty. ADR 1237 (1238 unused).

## The map's population is now the ledger's, and it is checkable with one grep

`doc/todo/65` was re-derived by reading all 80 `partial` and `reported` notes. Every live open row is
in exactly one bucket; every bullet whose row is no longer open is gone. Membership now agrees with
`grep 'status =' doc/conformance/ledger.toml` by construction, which it did not: the aggregate list
carried §7.6.4, §8.6.6, §10.4 and §10.4.2 after all four went `implemented`, and six `departed` rows
sat in buckets as though they were owed work.

Moved on a current note: §10.7 is an aggregate and §10.7.4 keeps hard-rendering for the clip region
no backend states; §12.3.5.1 and §12.6.4.6 are not-owed; §12.7.8.3.3 is in one bucket instead of two;
§11.6.5.2 is bucket 6's only member, a bound on a decoded grey plane costing 27.2 MB corpus-wide;
§11.5.3 joins the four-component family; §12.7.8.3.2's residue is `/APRef` alone. Two premises left
the expired list built — ADR 1107's pricing (1206), ADR 1113's last arm (1218).

## Two sentences that outlived their code

`pdf-signature`'s crate header said the program "has no trust store and no network, so §12.8.1's
third question is not answered", under a heading reading *Nothing here is called valid*. `trust.rs`
answers it under host-supplied anchors and `verdict.rs` is the type that says the word. Rewritten.

ADR 1219's signature policy reached `pdf-signature` and not the program. `viewer_core::notes::
signature_policy` now says which policy a signature was made under, whether the copy the file
carries is the one the signer committed to — and why a mismatch is not an alteration — and reads out
ETSI EN 319 122-1 clause 5.2.9.2's user notice, which that clause means to be shown whenever the
signature is validated. Paraphrased under ADR 1085, never quoted.

## Handed on, not taken

The `rejected rather than deferred` sweep: two occurrences, one row (§12.7.5.3), both naming a party
and both expired — `ViewState::choose_file` does the refused thing under `may_choose_file`. The
sibling family `refused rather than <verb>` gave 13 occurrences over 11 rows, 6 naming a party, two
more expired (§12.8.2.2.2, §7.5.6). Three stale ledger sentences found beside them: §12.11.6's first
sentence still says `partial` on an `implemented` row, and §8.11/§8.11.1/§8.11.4 and §12.5 still rest
on "an operation this program does not perform" and "there is no print path".
