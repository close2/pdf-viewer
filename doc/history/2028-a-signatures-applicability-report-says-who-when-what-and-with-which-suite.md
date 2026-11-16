# 2028 — A signature's applicability report says who, when, what and with which suite

Session 2028, a single ledger round, the signatures lane (`doc/questions/A800` item 3, `doc/todo/77`
item 3; §12.8.3.4.5). ADR 2872; Q855.

**The slice, and why.** TS 119 172-4 REQ-4.5-01's report beyond the level ADR 2866 built, and
REQ-4.2-03 d) and e): the rest of what A800's "tell the user the levels" owes a reader, all of it
inside `pdf-signature` and `viewer-core`. Round 2025's open end, a fixture reaching `TOTAL-PASSED`,
came with it. Item 5 (`TAsInTrustedList`) was not started: it reaches the policy readers of both
syntaxes and the host's lists through `notes`, past what the budget left after the gates.

**The premise.** Held: `notes` said the level and nothing of REQ-4.5-01 b) to h); no `A*` after A800
cites the item. Two parts did not: a)'s scope text cannot be reproduced under ETSI's notice
(ADR 1085), so Q855; h)'s assessment needs ETSI TS 119 312, whose fetch answered 403.

**Built.** `verdict::applicability`: `Report` with a)'s `Scope`, b)'s subject attributes and
`subjectAltName` names, c)'s `pseudonym` (`2.5.4.65`), d)'s claimed signing times (`signingTime`,
`/M`), document and signature timestamps (policy, accuracy, established or not, EU qualification
not determined), the material's revocation, issue and next-update times about the signer, the best
signature time; f)'s attributes by set with a commitment named; `Cryptography` for h) and
REQ-4.2-03 e). `timestamp::Claim::policy`. `notes` words one sentence per item beside a listed
level, and e)'s warning wherever lists were given.

**Tests.** 7 in `verdict/applicability/tests.rs` (an openssl certificate with a pseudonym and two
alternative names, made under `r2028/gen/`); 2 in `verdict/level/tests.rs`, a listed signer of a
generated root whose CRL was issued after the best signature time (`TOTAL-PASSED`, Annex A advanced)
and the same with an earlier CRL (not); 1 in `notes`. **Plants**: the pseudonym's identifier, an
unsupported curve dropped, the scope's "paraphrase": 4 tests failed; reversed, pass.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-signature`, `viewer-core`): 0. nextest
of the two: 1104 of 1104, 7 skipped. Fuzz `cargo check --bins`: 0. Rustdoc `-D warnings
--document-private-items` on the two: 0. Conformance after `git add -N`: 472 passed.
Tier 2 (load 85 at the start): `corpus` 101, 974 documents, 61 incomplete, as at HEAD;
`raster_golden` 101, held 951, moved 23, the same 23 as round 2025's; `selection_census` 0 (974);
`accessibility_census` 0 (1020). `script_corpus` not owed (no script reached); `launch_path` not
run. `state.sh --round 2028 docs` 0.

**Habit, and what is left.** A clause that requires a text printed is a clause whose text needs a
licence; ask before carrying it. §12.8.3.4.5 stays `implemented`. `doc/todo/77`: a)'s text (Q855),
h)'s TS 119 312 assessment, then the fetch, `TAsInTrustedList`, a C host's lists, TSAs.
