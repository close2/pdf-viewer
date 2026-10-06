# 1580 — The reader's policy words are every window's

Session 1372. Status: **accepted**. Closes `doc/todo/30`'s first item; amends nothing.
Context: ADRs 1039 (`Command::Trust`), 1101 (`Command::References`), 1106 (`Command::Audience`),
1539 (the document opens on a thread beside the window), 0737 (a confined worker is resumed).
Code: `crates/viewer-host/src/reader.rs` (`ReaderWords`, `ReaderSupply`), the argument loops of
`quorra`, `quorra-gtk`, `quorra-qt` and `quorra-confined`, `viewer_gtk::Opening::start`,
`viewer_qt::Host::open`, `tools/drive-windows.sh` (`reader_words`, steps 34 to 36).

## 1. What is sent, and when

The three commands are the reader's, not a document's: each applies to every document opened
afterwards, until sent again (`viewer_core::Command`'s own documentation for all three). So a window
sends them **once, before its first `Command::Open`**, on the thread that opens the document —
`quorra`'s `open_document`, GTK's `Opening` thread, Qt's opening thread, and the confined window's
`anticipate` thread. Before the document, for `Command::Restrict`'s reason: the audience decides
which layers the first interpretation draws, and a verdict worded under no anchor would be worded
again. On that thread rather than the window's, because reading two directories is nothing page one
needs (`CLAUDE.md` principle 2). Every later open in the two native windows goes to the same
`Viewer`, which keeps the three values, so `opening_commands` does not repeat them.

## 2. The confined window

Its wire already carried all three (`viewer_confined::protocol`'s `TRUST`, `REFERENCE_FILES` and
`AUDIENCE` tags), with the certificates and the target documents as bytes, so the host reads the
directories on its own filesystem and the worker reads none. A worker that replaces a dead one is a
new viewer, so the host keeps the words and sends their commands again before the reopen: a window
whose second worker lost its anchors would judge a signature differently after a crash.

## 3. What the drive shows, and what the brief assumed

Each word is launched with it and without it in all four windows. Trust: a document this drive signs
(`adbe.pkcs7.detached`, a signer certified by a root the drive issues) is called valid only where
`--trust-anchors` names the root and `--accept-unknown-revocation` is given, because the file
carries no §12.8.4 material; without the word the report says the third question was not asked.
References: a reference `XObject` draws the supplied blue page, and its grey proxy without the word.
Audience: a group whose Table 100 `/User` names Ada is drawn for `--reader-name Ada` and not for
`--reader-name Bob`. **The brief's audience observable — the reader's name in a new annotation's
`/T` — does not exist**: `Command::Markup`'s documentation says `/T` is not written, on the ground
that a person's name is not something this program knows, and §8.11.4.4's answer is about which content a
reader is shown, not who authored a mark. Whether `--reader-name` should also become Table 170's
`/T` is a separate decision about authorship and is not taken here; the drive witnesses the word by
the clause it serves.
