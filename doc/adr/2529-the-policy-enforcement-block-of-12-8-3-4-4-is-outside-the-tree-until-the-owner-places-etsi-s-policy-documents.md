# 2529 — The policy-enforcement block of §12.8.3.4.4 is outside the tree until the owner places ETSI's policy documents

Status: deferred to `doc/questions/Q713`. Session 1847, a clause round of batch one hundred and twenty-five.
Context: the owner's question of 2026-10-27 ("What is open, what is holding us up?"), ADRs 1085 (a free
specification is fetched), 1219, 1709, 1728; ETSI EN 319 122-1 clauses 5.2.9.1 and 5.2.9.2 and EN 319 122-2 cited by
clause and never quoted (their notice forbids reproduction).

## The question

Is §12.8.3.4.4's last requirement on the explicit-policy profile — that a validator enforce the signature policy's
constraints — in reach: is there a freely acquirable syntax a signature can name, whose constraints this tree could
enforce over a synthetic signature?

## What was found

- **No signature in reach names one** (ADR 1709, the signature census): the policy the corpus signs under is a
  human-readable PDF, bound and shown (ADR 1728) and not enforceable.
- **RFC 3125** (Electronic Signature Policies, ASN.1 syntax, category Experimental) is free and was fetched. It could
  be parsed and its signing period, mandated attributes, algorithm constraints and commitment rules read.
- **The document ETSI's two ENs point to for signature policies is TS 119 172-1** (EN 319 122-1 and EN 319 122-2
  each place it in their informative references beside the policy identifier). It and ETSI TR 102 272 were not
  acquired: ETSI's server answers this tree's client with HTTP 403 for them and for EN 319 122-2, which ADR 1085
  fetched. Presenting the client as a browser to pass that check was not tried; it would be circumventing a block.
- A policy's trust conditions are decided over certificates, which needs the host-supplied anchor (ADR 1076), so
  even RFC 3125's constraints are only partly enforceable without one.

## Decision

Build nothing. RFC 3125 alone would leave the row `partial` (the clause's profile points at TS 119 172-1, which
would still not be read) and enforce a syntax no signature in reach uses; the row's note names the block as outside
the tree in one sentence. The owner's word is asked in Q713: placing TS 119 172-1 to -4 and TR 102 272 in `doc/`
(untracked, as the other ETSI texts are) opens the build, with synthetic fixtures since no signature on disk can
exercise it. The row stays `partial`. Reopening trigger: those documents are held, or ETSI's server answers.
