# 2822 — A recipient key is the reader's, held for the window, not sent with each open

Session 2002, a ledger round (§7.6.5.1). Status: implemented. Rows: §7.6.5, §7.6.5.1.
Carries out ADR 2801's "What is left" item 1 and ADR 1134 decision 2, under the owner's A800.

## Context

§7.6.5.1 has the reader "scan the recipient list for which the content is encrypted and ...
attempt to find a match with a certificate that belongs to the user." Sessions 1995 to 2001 built
the scan and the opening (`pdf_syntax::RecipientKey`, `Document::open_with_recipient`), and no
host called it: which certificate belongs to the user is the one input no file can supply, and
ADR 1134 decision 2 said where it comes from — a host, through `viewer_host::policy`, carried as a
secret on a command into the confined worker, defaulting to none.

The brief and ADR 1134 both say "on the open command". `Command::Open` is built in 261 places
across the tree, three of them hosts a parallel round was editing, and a field added to it is a
change to every one.

## Decision 1 — a policy command, `Command::Recipient`, held by the viewer

The key is a fact about **who is reading**, like `Command::Trust`'s anchors and
`Command::Audience`'s names: it opens every document enveloped to that reader, where a password
opens one file. So it is `Command::Recipient(Option<RecipientSecret>)`, sent once before the first
open (the reader's words become a fifth command beside Trust, References, Audience and Scripts),
held by `Viewer` until another or `None` arrives, and read by every open. It crosses into the
confined worker as its own frame (`RECIPIENT`, command code 53), before the open it serves — the
same channel as the open command, so ADR 1134's "crosses into the confined worker" holds. Every
host that reads `ReaderWords` reaches it with no edit of its own.

## Decision 2 — `RecipientSecret`, not `Secret`

`viewer_core::Secret` is §7.6.4.1's password: text typed a character at a time, reserved at the
standard's 127 bytes, counted in characters for a prompt's bullets. A key is DER, not UTF-8, read
whole, and paired with a certificate. `RecipientSecret` carries the two of `Secret`'s obligations
that apply: its `Debug` prints two lengths and no byte, and its key buffer is zeroed on drop with
the same best-effort `black_box` (no `unsafe`, no `zeroize`, for `Secret`'s reason).

## Decision 3 — the key is read only after the file asks for one

`Open::read` opens with the password as before; only `SyntaxError::RecipientRequired` with a key
held makes it build `pdf_syntax::RecipientKey` and open again with it. A document that is not
encrypted to recipients never touches the key, and pays nothing for one being held (principle 2).
A public-key document pays one second read of its cross-reference table.

## Decision 4 — what is said

No match, with or without a key, is `Event::RecipientRequired` — §7.6.5.1's prompt, not
`OpenFailed`, as `PasswordRequired` is §7.6.4.1's. A certificate that matches and a key that does
not open the envelope is `OpenFailed` with `SyntaxError::RecipientKey`'s words: another prompt
would not help it. The windows say `RecipientRequired` as a document that could not be opened,
naming `--recipient-certificate` and `--recipient-key`: a chooser for two files is a user interface
`doc/todo/38` does not build until the owner asks (`--trust-anchors`' rule). A host reads the two
files (DER, or PEM `CERTIFICATE` and `PRIVATE KEY` / `RSA PRIVATE KEY` / `EC PRIVATE KEY`),
refuses an `ENCRYPTED PRIVATE KEY` by name, and checks the pair with `RecipientKey::new` at launch
so that a key that cannot be used is said then. The C ABI gains `quorra_recipient_key` (bytes, a
null certificate withdraws), `QUORRA_EVENT_RECIPIENT_REQUIRED` (31) and `QUORRA_READER_RECIPIENT_*`
(6, 7) for `quorra_settings_reader`.

## Cost

The key lives for the window rather than for one command. Three copies are not cleared: the
confined frame's buffer, as a password's frame is not; the base64 decoder's scratch on the host,
whose output moves into the secret; and the `pdf_pki::key::PrivateKey` each open parses from it,
whose big integers are `pdf-pki`'s and outside this decision.
