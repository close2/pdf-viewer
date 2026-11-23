# 3487 — An imported page's layers are the state built for it

Session 2196. Status: accepted. Context: review 2195 (finding 2195-B02); ISO 32000-2 §8.10.4.1,
§8.11.4.4, §8.11.4.5, Table 100.

`Interpreter::enter_imported` builds a `ViewState` of the target document and carries onto it what
is the reader's or the operation's rather than either file's. The interpreter's optional content
for the span of the imported page is that state's `optional_content()`, reapplied by the setters
under the carried factors — not a fresh `OptionalContent::read`, which applies only the `View`
event at an unstated magnification and no audience. So a printed page prints its imported page's
layers under the `Print` event (§8.11.4.5), `/Zoom` answers at the reader's magnification, and
`/Language` and `/User` at the reader's audience, which is now carried with the purpose, the
magnification, the separation simulation and the null-font substitute.

What is not carried stays as it was: the containing document's manual group changes,
§12.6.4.13's actions and its overrides are filed under the containing document's object numbers
and mean nothing in the target (§8.11.2.2 identifies a group by which object it is). The target's
own initial state, §8.11.4.5's steps a) and b), is the one its `ViewState::of` reads.
