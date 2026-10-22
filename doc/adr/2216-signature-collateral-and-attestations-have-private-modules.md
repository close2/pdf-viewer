# 2216 — Signature collateral and attestations have private modules

Session 1691, docs. Status: accepted; implemented and scoped gates passed.

The signature root combined signature arithmetic and permissions with two
standalone object-graph readers. ISO 32000-2 §12.8.4.3 defines the document
security store and §12.8.7 defines legal content attestations. Each already
had its own types and helpers; neither needs the larger Signature implementation.
Moving these concerns gives readers a direct location without changing behavior.

The private signature/security_store.rs module owns SecurityStore, Vri,
StoreRefusal, StoreEntry, MAX_STORE_ENTRIES, their implementations and the
security_store, security_store_entries and streams functions. The private
signature/legal_information.rs module owns Legal, its implementation, legal
and census. Explicit root re-exports retain all nine existing public names.
No field visibility, limit, error text, attribute or runtime assertion changes.
The helpers remain private and the child modules do not enter the public API.
The security-store file uses the private module binding collateral so existing
function links named security_store remain unambiguous without rewriting docs.
The root's Arc import moves with its only users; other imports remain in place.

All original tests remain in the root, reading the same public paths through
the re-exports. Dependent caller compilation covers the viewer and converter.
The larger cryptographic implementation, trust results and verification policy
are outside this mechanical move. No bug found while moving is repaired here.
The file remains large; this is one bounded concern split rather than a claim
that line count alone establishes a better design.
