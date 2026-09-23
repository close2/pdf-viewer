# 1291 — A form is sent by `ureq` over `rustls` from the host, and its level is read once

Session 1227. Status: **accepted**, with section 4's cost put to the owner (`doc/questions/Q130`).
Context: `crates/viewer-host/src/submit.rs` (new), `crates/viewer-host/src/policy.rs`
(`SUBMIT_SCHEMES`, `Submissions`, `Sending`, `may_submit`, `asked_to_submit`),
`crates/viewer-host/src/restriction.rs` (`MACHINE`, `SUBMITTING`, `Row::Machine`, `Row::Act`,
`Row::Sending`, `Restrictions::send`), `crates/viewer-core/src/{command.rs,viewer.rs}`
(`Command::Respond`), `crates/viewer-confined/src/protocol.rs`, the three windows' `Event::Submit`
arms and restriction menus, `Cargo.toml`, `crates/viewer-host/Cargo.toml`, `deny.toml`,
`doc/stack.md`. Builds: ADR 1062 (the policy seam), ADR 1155 (the Links levels and their
direction), ADR 1275 (a document opened beside). Answers: `doc/questions/Q98`, whose answer the
owner gave on 2026-09-22 ("Otherwise I concur with your recommendations").
Clauses: ISO 32000-2 §12.7.6.2, §12.7.8.1, Table 246.

## 1. What was built

`viewer_host::submit::transmit` sends the request `pdf_model::submission::compose` made — method,
URL, media type, body — unchanged, blocking, with `ureq`. `Submitter` runs each submission on a
`submit-form` thread of its own and hands the answer back on a channel; the window collects it on
its own timer (GTK a `glib` timeout, Qt the drawing timer's `drawing_wait`, `viewer-ui` a wake of
its `winit` proxy). **Never the event thread** — a request waits as long as the server does — and
**never the drawing thread**, whose contract is that a draw can be taken back (`crate::drawing`); a
request on the wire cannot. The renderer and the confined worker are untouched: `quorra-confined`
asks `may_submit` at `refuse`, for `Links::Refuse`'s reason, and `Command::Respond` is `Uncarried`
on its wire.

## 2. The four levels, their words, and the one place they are read

`viewer_host::Submissions` is `refuse`, `ask` (the default, the owner's), `warn`, `send` —
`CLAUDE.md`'s four levels in ADR 1155's direction, **not** `off`/`on`/`ask`/`warn` as `Q98` wrote
them. `Q98` listed the restriction vocabulary before anybody asked which end is permissive; in the
same menu as the restrictions, `off` would mean *nothing is sent* here and *everything is let
through* two lines up. The four levels are the owner's; the spelling is ADR 1155's rule applied.
`may_submit(submission, level)` is the one place the level is read, and it answers in `Sending`'s
four arms. The level is **global**, held in `viewer_host::Restrictions` beside the window's
policy, and the menu in all three windows gains a third group, *What a document may ask this
machine to do*, whose one act is *sending a form*. It sends the viewer nothing. No command-line
word was added; the menu is the surface the owner named.

## 3. Before the level: the scheme

`SUBMIT_SCHEMES` is `http` and `https`, checked before the level in `may_submit` and again in
`transmit`, `LINK_SCHEMES`'s shape. Table 239's `/F` is "[a] URL file specification … giving the
uniform resource locator (URL) of the script at the Web server that will process the submission"
and Table 240's bit 4 is "an HTTP GET request", so a URL naming no Web server is outside the
clause; `file` is out for ADR 1155's reason and `mailto` because no handler takes an entity body.

## 4. The dependency, priced

Measured with `cargo tree -p viewer-host -e normal --prefix none`: 117 packages before, 134 after.
The 17 new to `viewer-host`'s tree, with licences from `cargo metadata`: `ureq` 3.4.2,
`ureq-proto` 0.6.4, `http` 1.5.0, `httparse` 1.10.1, `base64` 0.23.1, `utf8-zero` 0.8.1,
`percent-encoding` 2.3.2, `itoa` 1.0.18, `rustls-pki-types` 1.15.1, `openssl-probe` 0.2.1 (a path
finder; it links nothing) and `getrandom` 0.2.17 — all `MIT OR Apache-2.0`; `bytes` 1.12.1 `MIT`;
`rustls` 0.23.45 and `rustls-native-certs` 0.8.4 `Apache-2.0 OR ISC OR MIT`; `rustls-webpki`
0.103.15 and `untrusted` 0.9.0 `ISC`; `ring` 0.17.14 `Apache-2.0 AND ISC`. The lockfile gained 19,
the difference being targets this program is not built for (`schannel`, `security-framework`,
`security-framework-sys`, `core-foundation` 0.10, `wasi`). `getrandom` is now three versions and
`core-foundation` two, both `multiple-versions = "warn"`. `cargo deny check`: advisories, bans,
licences and sources ok, **no exception added**. Features: `ureq` with no defaults and
`rustls-no-provider` alone — no `gzip`, no `webpki-roots` (CDLA-Permissive-2.0, not on the list) —
and the provider named through `rustls`'s own `ring` feature rather than `ureq`'s `_ring`. The
roots are this machine's store, because which certificates a person trusts is their setting.
`openssl`, `openssl-sys`, `native-tls` and `webpki-roots` are banned in `deny.toml`.

`quorra --release`, built from this commit's parent and from it alone in copies of the tree with
one target directory: **23 970 328 → 25 826 432 bytes, +1 856 104 (+7.7 %)**.

**The cost that is not a number.** `rustls` needs a cryptographic provider and every stable one is
C: `ring` (C and assembly from BoringSSL, compiled by `cc`) or `aws-lc-rs` (larger, and C too);
`rustls-rustcrypto` has no stable release. `CLAUDE.md` principle 3 says a C dependency "must be
isolated in the sandboxed process and justified in writing", and `ring` runs in the unconfined
host, over bytes a server chose. The writing is here: the owner ratified `ureq` with `rustls`, which
cannot be built without one of the two; the host already links GTK and Qt, which are C and C++; and
the rule's examples are codecs over *document* bytes. That is an argument, not the owner's word, so
`Q130` asks it, recommending the exception be written into principle 3 until a pure-Rust provider
is stable. Nothing here waits on the answer except where TLS runs.

## 5. What an answer comes to

§12.7.6.2 states no duty about the response: its only word is a NOTE, "Presumably, the URL is the
address of a Web server that will process them and send back a response." The one `shall` is
Table 246's `/Status`, "[a] status string that shall be displayed indicating the result of an
action, typically a submit-form action". So a 2xx FDF answer (`application/vnd.fdf`, §12.7.8.1's
own type, or `application/fdf`, IANA's name for it) or XFDF answer goes to `Command::Respond`, which
imports it through the reader §12.7.6.4 uses into **the document that sent the form**, whether or
not it is in front, and the `/Status` is displayed there. A 2xx PDF is kept in a new file, mode
0600, in `$XDG_RUNTIME_DIR` or the temporary directory, and opens beside through
`viewer_host::Arrivals`, because every window opens a tab from a path. Anything else is said with
its status and media type. Redirects are not followed and are reported with their `Location`; the
answer is bounded at 64 MiB and the exchange at 60 s, both documented choices.

## 6. What a later round should not re-open

The vocabulary (section 2), the thread (section 1), the scheme order (section 3), and that an error
status imports nothing. What is still owed is section 4's question and Qt's window driven under
Xvfb; `quorra` and `quorra-gtk` were.
