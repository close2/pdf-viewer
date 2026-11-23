# 3485 — A group with no effect drops out of a visibility expression, and a malformed one is said as itself

Session 2196. Status: accepted. Context: review 2195 (findings 2195-B01, D01, D02); ISO 32000-2
§8.11.2.2, §8.11.2.3, Table 97.

## Decision

§8.11.2.3: a group whose intents the configuration's do not cover "shall have no effect on
visibility", and §8.11.3.2 gives a group the properties dictionary never listed no part either.
`OptionalContent::evaluate` answers such a group `None`, and:

- `/And` and `/Or` combine only the operands that take part;
- a `/Not` whose operand takes no part takes no part itself;
- an expression with nothing left shows the content, as Table 97's `/OCGs` route does when its
  set of states is empty.

Equating the group to ON — what the tree did — is the same answer only under `/And`; under `/Or`
and `/Not` it lets a group with no effect decide the result, and Table 97's two routes then
disagree about the same groups.

A `/VE` the clause gives no reading — a first element that is not an operator name, an operator
other than And, Or and Not, a `/Not` with no operand — is `Visibility::Malformed` with its own
sentence, drawn and reported as the depth bound is (the drawn-not-hidden choice is
`Visibility::TooDeep`'s), and never reported as the bound. A malformed operand anywhere in the
tree makes the whole expression malformed: no partial answer is invented from the rest. A `/Not`
with more than its one operand keeps its reading of the first, which is what a file meant by the
only operand the clause allows; it is not reported.

## Rejected

Reading a non-participating group as OFF: that gives it an effect as surely as ON does, and
`[/Not g]` would then show content the `/P` route's `/AllOn` over the same group shows too —
still the right answer by accident in one operator and wrong in the other.
