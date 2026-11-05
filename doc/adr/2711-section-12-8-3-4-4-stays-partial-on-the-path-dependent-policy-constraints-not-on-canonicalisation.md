# 2711 — §12.8.3.4.4 stays `partial` on the path-dependent policy constraints, not on canonicalisation

Status: accepted. Session 1948. Context: ISO 32000-2 §12.8.3.4.4 ("During validation conforming signature handlers
should not enforce seed values constraints, if present, but shall enforce signature policy constraints."); ADRs 1035,
1076, 2560, 2604, 2624, 2644, 2710; ETSI TS 119 172-3 clauses 6.7 to 6.10.

## The premise that did not hold

The brief, and the row's note before this session, named two things holding the row `partial`: human-readable
policies, a limit, and the two canonicalisation methods, the work. ADR 2710 builds the methods, so the brief asked
whether the row now takes `implemented` with a documented residue under ADR 2624's rule. It does not, because the
note's own list of constraints *named and not evaluated* still holds work. `policy_rules`' module comment (ADR 2560)
gives the reasons, and two of them are not limits:

- **The trust conditions of clauses 6.7 to 6.9** are not evaluated, the module comment says, because trust points
  are the host's anchors under ADR 1076 rather than the policy's. ADR 1076 decides where the anchors for §12.8.3.4.5's third question come
  from. It does not decide the policy's trust condition, which is a constraint the policy states with its own trust
  points, and §12.8.3.4.4 says a handler "shall enforce" it. `trust` already builds a path to supplied anchors, so
  building one to the policy's is work.
- **The certificate-issuer algorithm constraints of clause 6.10 and the requirements that reach the full certification
  path** wait on the same path.

The rest of that list is a limit or is outside validation: verifier rules ask a verifier to add attributes, rules scoped
to generation or augmentation alone do not apply to a validator, and an extension's syntax is whatever its identifier
names.

## Decision

The row stays `partial`. Its note names the residue as the human-readable policy, which is a limit, and the
path-dependent constraints above, which are owed. This is a reading of the residue ADR 1035 asks for, not a new
decision against anything. The round that evaluates a policy's trust conditions against its own trust points is the one
that can apply ADR 2624's rule again.
