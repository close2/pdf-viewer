# 2724 — RFC 5280's policy tree and name constraints are evaluated in the path validation

Status: accepted. Session 1955. Context: ISO 32000-2 §12.8.3.4.4 ("… but shall enforce signature policy
constraints.") and §12.8.3.4.5 (b); RFC 5280 sections 4.2.1.4 to 4.2.1.6, 4.2.1.10, 4.2.1.11, 4.2.1.14, 6.1 and
7.1; RFC 3125 sections 6.6.1, 6.8 and A.1; ETSI TR 102 272 clauses 6.6.1 and 6.8; ETSI TS 119 172-2 clauses
4.22.3 to 4.22.5 and 4.25.1; ETSI TS 119 172-3 clause 4.22 and Annex A; NIST PKITS sections 4.8 to 4.13; ADRs 1039,
1067, 1076, 2714, 2720, 2721. The ETSI texts are cited and paraphrased (ADR 1085); RFC 5280 and RFC 3125 are held in
`doc/md/rfc/`.

## What was there

`trust` validated a reduced path: it fixed RFC 5280 section 6.1.1's policy inputs to their defaults and refused,
as an unrecognised critical extension, every certificate carrying `certificatePolicies`, `policyMappings`,
`policyConstraints`, `inhibitAnyPolicy`, `nameConstraints` or a critical `subjectAltName`. A signature policy's
trust point that stated an acceptable policy set, name constraints or policy constraints, and a time-stamp trust
condition that stated the authority's name constraints, were named and not evaluated (ADR 2721).

## The construction

- **`x509` carries the six extensions undecoded** and recognises them; `trust::policies` and `trust::names` decode
  each where section 6.1 processes it, so a malformed one refuses the path by name.
- **`trust::policies::PolicyTree` is section 6.1.2 (a)'s `valid_policy_tree`**, one vector of nodes per depth, a
  node naming its parent by index and deletion marking rather than moving. `grow` is 6.1.3 (d), `map` is 6.1.4
  (b), `intersect` is 6.1.5 (g)(iii). `trust::constrained::State` holds the three counters and calls the steps in
  the section's lettered order from `Search::walk`; the key, name and length steps stay where they were.
- **The output is reported in the trust anchor's domain.** Below a mapping a leaf's `valid_policy` is a
  subject-domain policy the caller never named; `admitted` climbs to the ancestor whose parent is `anyPolicy`, the
  member of the `valid_policy_node_set` that 6.1.5 (g)(iii) intersects. PKITS states its expected sets that way,
  and the first reading (leaves as written) failed ten of its mapping cases.
- **`trust::names::NameState` keeps the permitted subtrees as a list of sets** and asks a name to lie in one
  subtree of each set that names its form. That is 6.1.4 (g)(1)'s intersection by definition: an intersection of
  two sets of subtrees has, in general, no spelling as a set of subtrees. Each form follows section 4.2.1.10's
  rule; a URI under a URI constraint that names no domain host is the rejection that section requires.
- **Section 7.1's comparison is exact on ASCII and undecided beyond it.** RFC 4518's preparation with case folding
  and white-space compression is ASCII lowercase and run-collapsing on ASCII text; its mapping and normalisation
  tables are not in this tree, so two values that differ and hold anything else are `Match::Undecided`, and a
  check that meets one refuses as this program's limit. Deciding either way would be wrong one way for permitted
  subtrees and the other way for excluded ones. Name *chaining* stays byte comparison, which can only fail to
  chain.
- **`PathInputs` is section 6.1.1's inputs (c) and (e) to (i)**, the three flags held as `SkipCerts` counts so a
  trust point's `PolicyConstraints` applies as though the trust point were certificate zero. RFC 3125 section
  6.6.1's example for `inhibitPolicyMapping` — a value of one lets the certificates the trust point's subject
  issued map — is that reading. `validate`, `validate_for` and `validate_path` pass the defaults, under which a path
  no certificate constrains validates as before; `validate_constrained` takes the inputs and returns the policies.
- **A trust point's members become the inputs.** RFC 3125's module and TS 119 172-3's Annex A are `EXPLICIT TAGS`
  and restate `NameConstraints` and `PolicyConstraints` there, so a policy's `[0]` wraps its value where a
  certificate's replaces the tag: `names::Tagging` says which. TS 119 172-2's XML gives each subtree's base as a
  base-64 `GeneralName`; its schema names the mapping member `InhibitExplicitPolicy`, which clause 4.22.4.1's
  second component makes the mapping one. An XML acceptable policy that is a URI and no object identifier names
  nothing a certificate can assert, and keeps the set named rather than evaluated. A subtree stating a distance
  keeps the whole constraint named, so no subset of it is judged as though it were all of it.
- **The authority's name constraints are judged on the authority's certificate only.** RFC 3125 section 6.8 calls
  them constraints on the name of the time-stamping authority, so `certification_path::authority_name` tests the
  first certificate of the authority's path with section 6.1.3 (b) and (c)'s test, not every certificate of it.
- **An acceptable policy set alone binds only with an explicit-policy requirement.** TS 119 172-2 clause 4.22.5.1
  and RFC 3125 call it the *initial* set, which is section 6.1.1 (c); section 6.1.5 succeeds with a NULL tree
  while `explicit_policy` is above zero. The sentence says when a path is valid for none of the set's policies,
  and Q796 asks the owner whether a stated set should also require one.

## Bounds (principle 3)

At most `MAX_POLICY_NODES` (1024) tree nodes for one path — the policy-mapping blow-up known against validators of
this algorithm is the case — `MAX_POLICIES` (32) entries per extension or set, `MAX_SUBTREES` (64) subtrees held
for one path, `MAX_NAMES` (32) names per certificate, and `MAX_RDNS` (32) per name. Past any of them the path is
refused as this program's limit, which a policy's verdict counts as not evaluated.

## The witnesses

NIST PKITS's sections 4.8 to 4.13 — 101 cases, 124 runs of section 6.1.1's inputs — are `trust::pkits`, one test
per case named by its number, over PKITS's own certificates in `trust/pkits/`; each expected verdict, and the
authorities- and user-constrained sets where PKITS states them, are PKITS's from RFC 5280. No corpus signature
states a machine-readable policy (ADR 1709), so the policy-level tests construct their policies.

## What it changes outside a policy

A certificate carrying one of the six extensions is now validated rather than refused by section 4.2's rule, so
the ordinary verdict changes for such a path, and a path whose own certificates' policies or names fail is now
refused by the step that fails it. The round compared the signature census over the corpora at HEAD and after.
