# 1581 — One reading of the words, not the toolkits' parsers

Session 1372. Status: **accepted**. Context: ADR 0604 (a word one window refuses), ADR 1580.
Code: `crates/viewer-host/src/reader.rs`.

## Decision

The reader's seven words — `--trust-anchors`, `--accept-unknown-revocation`, `--reference-files`,
`--reader-name`, `--reader-title`, `--reader-organisation`, `--interface-language` — are read by
`viewer_host::ReaderWords::take` in every window, and `quorra`'s own parsing of them is replaced by
it. `GApplication`'s option entries and Qt's `QCommandLineParser` are not used.

## Why

- **Levelness.** ADR 0604 is the record of two windows of three refusing a word the third took. A
  toolkit's parser would give each window its own spelling rules — each toolkit has its own
  conventions for `=`, for single-dash forms and for abbreviations — and its own refusals, so the same command line could
  mean different things in different windows. One function has one meaning.
- **The windows already parse by hand.** `quorra-gtk` runs its application with no arguments
  (`run_with_args(&[])`) and `quorra-qt` hands Qt none; every other policy word is `viewer_host`'s
  constant matched in a plain loop. A toolkit parser for three words would be a second parser beside
  the first.
- **The refusals are the program's.** A directory that is not one, or a word with nothing after it,
  stops the launch with a sentence (`take`'s `Err`), the rule `quorra` already kept; a toolkit's
  generic message would not name why a launch that ignored the word would mislead.

## Cost

A window's `--help` is its own usage line, not a toolkit-generated one; `ReaderWords::USAGE` is the
words' part of it, so the four usage lines cannot drift apart.
