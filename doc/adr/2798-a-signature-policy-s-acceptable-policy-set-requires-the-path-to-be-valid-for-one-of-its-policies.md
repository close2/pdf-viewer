# 2798 — A signature policy's acceptable policy set requires the path to be valid for one of its policies

Status: accepted. Session 1993. Context: ADR 2724; `doc/questions/Q796`, answered by the owner in
`doc/questions/A796` ("option 2"); RFC 3125 section 6.6.1; ETSI TS 119 172-2 clause 4.22.5.1; RFC 5280 sections
6.1.1, 6.1.3 and 6.1.5. The ETSI text is cited and paraphrased (ADR 1085).

## The question and the answer

RFC 3125 section 6.6.1 and TS 119 172-2 clause 4.22.5.1 make a trust point's acceptable policy set the initial
set of certificate policies, each acceptable under the signature policy. ADR 2724 passed it to RFC 5280 section
6.1.1 (c) as `user-initial-policy-set`. Under section 6.1.5, though, a path whose certificates assert no policy
still succeeds unless an explicit policy is required. So a stated set bound nothing on such a path. Q796 asked
whether a stated set should also require the path to be valid for one of its policies. The owner answered option
2: yes.

## The construction

`certification_path::implied` gives a trust point that states an acceptable policy set section 6.1.1 (f)'s
`initial-explicit-policy`, held as `requireExplicitPolicy` 0. That is the strictest count, so it replaces any count
the trust point's own `PolicyConstraints` states, and every search to the trust point is validated under it: the
signer's, the time-stamping authority's and an attribute authority's. A path valid for none of the set's policies
is therefore refused by section 6.1.3 (f) or 6.1.5 (g), and the condition is broken. Where the trust point mandates
a path, clause 4.22.5.1 item 5 still gives that path preference over the set, as ADR 2781 has it.

Every sentence that names the set says how it is read. `ACCEPTABLE_POLICY_SET` names it as an acceptable policy set
that this program reads as requiring the path to be valid for one of its policies, so the reading appears wherever
the trust point's evaluated or yielded members are listed.

This is a reading the owner chose, not a requirement section 6.1.5 states. A policy that wanted only RFC 5280's
literal behaviour has no way to say so, and that cost is accepted.
