# 2156 — The reader's other words are read once beside the settings, and cross the settings handle

Session 1660, batch one hundred and two's slot 6, a host round. Status: **accepted** and **built**.
Context: ADRs 1580 and 1581 (`ReaderWords`), 1954 (`--archive`), 2120 (the settings handle), 0247
(the ABI's shapes); RFC 0010 section 5.

## 1. What was left

ADR 2120 made the seven settings words one function and a handle, and left the reader's other words
— `--trust-anchors`, `--accept-unknown-revocation`, `--reference-files`, the three `--reader-*`
words, `--interface-language`, `--archive` — out of a C or .NET window's reach: `quorra_window`
opened with no anchor, no target document, nobody reading and the chooser for a conversion. And
the scripts level lived twice: `ReaderWords::scripts` and `Settings::scripts`, which `quorra-gtk`
and `quorra-qt` copied one into the other at launch and `quorra_window` copied back the other way.

The brief's premise held for the ABI and not for the binaries: `ReaderWords::take` was already the
one reading the four windows call. What the four copied was the second call beside it and the level
carried between the two halves.

## 2. Decisions

1. **`viewer_host::policy::Words`** holds the `Settings` and the `ReaderWords` and is the one reading
   of a command line's words: `take` asks `ReaderWords::take`, then `Settings::take`, and answers
   `Taken::Reader` or `Taken::Setting(constant)`, so `quorra-confined` still refuses by name the
   settings words it has no level for. The four binaries call it; `into_parts` hands the window its
   settings and the document thread its reader's words. The scripts level is written in both
   halves by `take`, `set` and `scripts`, and in no other place.
2. **A typed setter reads the value as the word would**: `Words::state(ReaderWord, value)` hands the
   value to `ReaderWords::take` after the word `ReaderWord::word` names, so a directory that is not
   one is refused in the command line's own sentence, by construction rather than by a second
   parser.
3. **Four entry points, six constants** (ADR 2120 section 3's rule: a constant is free, an entry
   point per word is not): `quorra_settings_reader(settings, QUORRA_READER_*, value)` for the six
   words with a value; `quorra_settings_accept_unknown_revocation(settings, bool)`;
   `quorra_settings_archive(settings, target, profile)` in `quorra_archive_target`'s and
   `quorra_archive_profile`'s indices, either negative for none; and
   `quorra_settings_take_next(settings, word, next, *used)`, so a host with a command line reads a
   word with its value as the four windows do. `quorra_settings_take` now reads every word and
   refuses one that wants a value; a word it used to leave (`--accept-unknown-revocation`) it takes.
4. **`quorra_window_open_under` opens under all of it**: the four commands are sent on the
   document's thread before the first `Open`, as the toolkit windows send them, and each file a
   directory holds that is not an anchor or not a target document comes back as a `SAID` record at
   the next `quorra_window_take_owed` (trap 5); the conversion is the archive key's.

## 3. Refused

- **An entry point per word** (`quorra_settings_trust_anchors` and five more): six functions saying
  one thing.
- **`--trust-anchors=<dir>` accepted by `quorra_settings_take`**: a spelling no window takes, which
  is a policy that exists in one build (ADR 1581).
- **The directories read when the handle is set**: what is not needed for page one is not read
  before it (`CLAUDE.md` principle 2); the handle keeps the path, as the command line does.

## 4. Costs

Four entry points (341 to 345 with ADR 2152's), six constants, one type in `viewer-host`. An audience
reaching a window from C is shown by the unit tests' equality with the typed words and by the
directory's refusals, not by a page drawn under it.
