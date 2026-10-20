# 2141 — A transform worker is given only what the person's command line names

Session 1652, slot 6 of batch one hundred and one. Status: **accepted** and **built** in
`crates/pdf-transform/src/confined.rs` (`Fences`), `src/pattern.rs` (`Pattern::admits`) and
`src/bin/quorra-transform/route.rs` (`fences`). Context: ADR 2140; `CLAUDE.md` principle 3 and
`doc/questions/A24` ("the renderer can still name no path"); ADRs 0880, 1155, `doc/rfc/0007` §4.

## The decision

A worker the document subverted can still ask its broker for anything, so every request is answered
through a fence drawn from the broker's own reading of the arguments, before the worker runs:

| the worker asks for | it is given it only where |
|---|---|
| a file (font, profile, configuration, `--attach` payload) or a document | the path is, as typed, a document, `--attach`, `--font`'s right half, `--config` or `--output-intent-profile` |
| a descriptor's line | the number is `--password-fd`, `--encrypt-owner-fd` or `--encrypt-user-fd` |
| an output | `-o` is `-`, or the name is one `Pattern::admits` — exactly the names `expand` can write: literals as typed, `%d`/`%p` digits, `%l`/`%t` what `sanitise` leaves |
| a file beside the document | `--resolve-external-data` was given and the path is one plain name in the first document's directory — ADR 1155's rule, checked again on this side |
| a tool run | the configuration's `derive` remedy at that site names that tool, and the request is the declaration field for field, its arguments filled with `{site}` and `{target}` as `request_for` fills them |
| standard output, error, the terminal | always: they are the run's own streams |

A refusal is an `io::Error` the program prints in its own words, so it is loud. **The worker names
paths, and that is the difference from A24's renderer worth stating**: it names only a key into a
set the person already typed, and a name outside it reaches nothing. A flag added to the grammar
that reads a file has to be added to `route::fences`, or the worker is refused it by name — the
direction a mistake should fail in.
