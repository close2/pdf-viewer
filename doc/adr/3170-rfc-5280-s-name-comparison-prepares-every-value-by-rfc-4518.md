# 3170 — RFC 5280's name comparison prepares every value by RFC 4518, not only ASCII ones

Session 2117. Status: accepted and built. Row: §12.8.3.4.4. Context: ADRs 2724, 2725, 2775,
2782, 3158, 3162; RFC 5280 section 7.1; RFC 4518 section 2; RFC 3454 tables A.1, B.2, C.3, C.4,
C.5 and C.8. RFC 4518 was fetched from the RFC Editor (`https://www.rfc-editor.org/rfc/rfc4518.txt`)
and is not tracked.

## Context

Round 2115's trap was to read the tables a row's note cites before moving its status. ADR 2725's
table named six pieces of work; ADR 2775 carried four of them forward and dropped one without a
word: "a distinguished-name value outside ASCII that differs from its constraint", whose section
7.1 comparison `trust::names` left undecided because RFC 4518's tables were not in the tree. ADR
3162's table, written from the note, did not list it, and the code still refused such a name. It
was work, so §12.8.3.4.4 could not move on ADR 3169 alone.

## Decision

`trust::names::string_prep` performs RFC 4518 section 2's six steps as RFC 5280 section 7.1 asks:
section 2.2's map in the section's own "complete list" of control and separator code points, with
RFC 3454 table B.2's case folding; Normalization Form KC; section 2.4's prohibitions by RFC 3454
tables A.1, C.3, C.4, C.5 and C.8 and U+FFFD; no bidi step; and section 2.6.1's insignificant space
handling for a stored value. The lists are written as printed rather than read from a later
Unicode's categories, because step 4 prohibits what Unicode 3.2 left unassigned and a newer
category would map some of those to nothing first. X.520 clause 7's preparation
(`policy_rules::signer_attributes::values::prepare`, ADR 3158) is not reused: it prohibits a
leading combining mark, RFC 4518 does not, and its unassigned test is today's Unicode rather than
table A.1.

A value the preparation fails on is RFC 4518's "Undefined" and stays `Match::Undecided`, refused by
name as before; a value of a string type this program does not decode (`TeletexString`, which
section 7.1 makes optional) is the other undecided case. `Straße` now matches `STRASSE`, which it
did not decide before; the trusted-list determinations that compare names through `texts_match`
get the same answer.
