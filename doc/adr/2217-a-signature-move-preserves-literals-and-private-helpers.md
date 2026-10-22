# 2217 — A signature move preserves literals and private helpers

Session 1691, docs. Status: accepted; implemented and scoped gates passed.

A line multiset alone can miss a missing helper, changed literal or accidental
reordering during a source split. The signature extraction therefore records
its original source hash and the two contiguous ranges before editing.
Both moved bodies retain exact bytes after removing only outer blank lines.
No module-relative path rebasing is needed inside an item.

A proc_macro2 token-tree comparison retains delimiter kinds, identifiers,
punctuation spacing and literal spelling. It separately compares literals and
the moved top-level item inventory: ten security-store items with 108 literals,
and four legal-attestation items with 72 literals, including documentation.
The reconstructed root differs only in the approved module declarations,
explicit re-exports, removed Arc import and removed item ranges. Its token
stream also matches the expected reconstruction, including the original tests.

Two scratch negative controls must fail: changing MAX_STORE_ENTRIES from 1024
to 1023, and removing its declaration. Both fail with exit 101. These are proof
calibrations, not runtime changes or modified test expectations. Compilation,
existing tests, linting and rustdoc still run after the lexical proof; identical
bodies cannot by themselves establish that imports and public paths resolve.
The proof source and receipts remain under scratchpad/r1691.
