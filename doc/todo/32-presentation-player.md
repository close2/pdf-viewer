# A presentation player

Status: the state machine advances; nothing animates.
Priority: 32
Clauses: §12.4.4
Code: `crates/viewer-core/src/viewer.rs` (`Command::Tick`), a host

§12.4.4's whole presentation is read — Table 164's transition styles, `/Dur`'s auto-advance,
§12.4.4.2's sub-page navigation — and since the hundred-and-fiftieth session the core *advances*
a slide show: `Command::Tick { millis }` is how a state machine with no clock is told the time
(ADR 0135), and `Event::Transition` names what should happen.

What is missing is a host that **draws** the frames of a named transition. The core will not do
it: rule 3 of §0 is that there is no clock in `viewer-core`, and a transition is an animation
over wall time.

Cheap, and a good first exercise for a native host (todo 30) — the platform's own animation
clock is exactly what `Command::Tick` was shaped for.
