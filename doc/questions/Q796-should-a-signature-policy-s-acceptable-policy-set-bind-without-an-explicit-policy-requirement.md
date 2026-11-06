# Q796 — Should a signature policy's acceptable policy set bind without an explicit-policy requirement?

Source: round 1955, evaluating a trust point's acceptable policy set as RFC 5280 section 6.1's input (ADR 2724).

## What the texts say

RFC 3125 section 6.6.1 and ETSI TS 119 172-2 clause 4.22.5.1 both call a trust point's acceptable policy set the
*initial* set of certificate policies, each acceptable under the signature policy. RFC 5280 section 6.1.1 (c)
names that input `user-initial-policy-set`, and section 6.1.5 makes a path succeed when the `valid_policy_tree` is
not NULL **or** `explicit_policy` is above zero. So a path whose certificates assert no policy at all validates
under a stated set unless something — the trust point's `PolicyConstraints`, or a certificate's — requires an
explicit policy.

## What this program does

It follows RFC 5280 literally: such a path is met, and the sentence says the path is valid "for none of the policies
of its acceptable set" and that RFC 5280 asks for one only where an explicit policy is required. A set stated
together with `requireExplicitPolicy` binds fully.

## The question

Should a stated acceptable policy set also require the path to be valid for one of its policies — that is, should
it imply `initial-explicit-policy` — on the reading that a policy author who lists acceptable policies means no
other path to be acceptable?

1. **No (the current construction)**: the set is RFC 5280's initial input and nothing more; a policy that wants
   an explicit policy says so with `PolicyConstraints`.
2. **Yes**: a stated set implies `requireExplicitPolicy` 0 for that trust point, and the sentence says the program
   reads it so.

Either answer is one line in `certification_path` and changes no ledger status.
