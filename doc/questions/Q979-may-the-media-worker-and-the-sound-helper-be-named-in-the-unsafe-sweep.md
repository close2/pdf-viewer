# Q979 — May `pdf-media-worker` and `quorra-sound` be named among the crates that lift `unsafe_code`?

Source: round 2157 (ADR 3329), found by its nextest of `viewer-qt`.

## What the round found

`viewer-qt`'s `tests/unsafe_position.rs`, `only_the_named_crates_in_the_tree_lift_the_denial`,
fails on main since `df503353` (the media worker) and `75457369` (the sound helper): it reads every
crate's `src` for an `expect`/`allow` of `unsafe_code` and finds two names its list does not hold,
`pdf-media-worker` and `quorra-sound`. Rounds 2150 and 2154 ran no `viewer-qt` test, so neither saw
it; it has nothing to do with this round's change, which adds no `unsafe`.

Each lift is the one ADR 1945 (`doc/questions/A430`) already admits for the three workers: the
binary's `keep_only_the_standard_descriptors`, called as `main`'s first statement, closes the
inherited descriptors in one `close_range`, under `#[expect(unsafe_code, reason = "… (ADR 1945)")]`,
while each crate's library forbids `unsafe_code`. ADRs 3297 and 3317 say the two programs do this
"for `pdf-media-worker`'s reason (ADR 1945)".

The test's own comment says a name past its list "is a change to a rule the project owner stated,
and it belongs in an ADR rather than in a commit that widens this list", and the round was stopped
from widening it. So it is asked.

## The question

May the sweep's list name `pdf-media-worker` and `quorra-sound`, as two more programs closing their
inherited descriptors under ADR 1945, with an ADR recording it?

## Recommendation

**Yes.** Neither lift reads a byte a document wrote — each runs before the program reads its input
— and both are the construction the owner already answered for in A430. The alternative, a shared
crate holding the one `close_range` that the five programs call, would move the lift to one name
the list could hold instead of five; it is a larger change for the same `unsafe` and could follow.

Until the answer, the failure stands on main and is reported by each round that tests `viewer-qt`.
