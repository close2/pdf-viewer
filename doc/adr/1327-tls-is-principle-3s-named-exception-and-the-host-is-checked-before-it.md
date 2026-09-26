# 1327 — TLS is principle 3's one named exception, and the host is checked before it enters

Session 1245. Status: accepted and **built**.
Context: `crates/viewer-host/src/submit.rs` (`check_url`, `UrlRefusal`, `NAME_OCTETS`,
`LABEL_OCTETS`, `TransmitError::Url`), `crates/viewer-host/src/policy.rs` (`may_submit`),
`crates/viewer-host/tests/submit.rs`, `CLAUDE.md` principle 3, `doc/stack.md`, `doc/PLAN.md`.
Answers: `doc/questions/Q130` — the owner's word of 2026-09-23 is in `A130`.
Amends: ADR 1291 section 4, whose cost was an argument awaiting the owner and is now the owner's ruling.

## 1. The exception

Principle 3 says a C dependency is confined and justified in writing. `ring`, the provider under
`rustls`, is C and assembly and runs in `viewer-host`, which is unconfined. The owner took option 1
of `Q130`: the exception is written into principle 3 by name — TLS for a transmission a person
allowed, `ring` in the unconfined host, revisited when a pure-Rust provider is stable — with one
condition, in the owner's words in `A130`: validation making sure only valid input is passed on, rudimentary
rather than precise. The revisit is a stable release of a pure-Rust `rustls` provider; option 2 of
`Q130` (a confined TLS helper) becomes the answer instead if a codec-style exploit class appears in
the provider.

## 2. What reaches which library

- **`rustls`** receives the URL's host: ureq hands it over for the handshake's server name and
  `rustls-webpki` matches it against the certificate. That host is Table 239's `/F`, a string the
  document chose, passed unchanged by `pdf_model::submission::compose`.
- **`ring`** receives no document-derived data: its inputs are the keys, signatures and transcript
  of the server's handshake. ADR 1291 section 4's ground stands.
- The path and query travel as application data inside the connection; the TLS stack encrypts them
  and parses nothing of them.

## 3. What is checked, where, and what is not

`check_url` runs in `transmit` after the scheme check, before the agent and the request exist, and
in `may_submit` after its scheme check, so a malformed host is refused at every level and never
asked about. Each failure is a `UrlRefusal` variant whose sentence names the check:

| check | bound | source of the bound |
|---|---|---|
| `//` and an authority after the scheme | present | RFC 3986 section 3.2 |
| user information (`@`) in the authority | refused | a form's address has no use for it |
| `%` anywhere in the authority | refused — also refuses an IPv6 zone identifier | a host is sent as written |
| host name length | at most 253 octets, one trailing dot admitted | RFC 1035 section 2.3.4 (255 on the wire) |
| label | non-empty, at most 63 octets | RFC 1035 section 2.3.4 |
| label characters | ASCII letters, digits, hyphen; a digit first admitted | RFC 1035 section 2.3.1, RFC 1123 section 2.1 |
| bracketed host | parses as `std::net::Ipv6Addr` | RFC 3986 section 3.2.2 |
| port, where written | one to five ASCII digits naming a `u16`; `+80` and an empty port refused | RFC 3986 section 3.2.3 |
| path, query, fragment | no control character, no white space | — |

**Not checked, deliberately**: a hyphen first or last in a label, an all-digit name that is no real
IPv4 address, and the path's grammar. None is a string the TLS stack parses, and a host of the
wrong shape but the right characters fails at the resolver or at the certificate's name check, each
of which already refuses by name. An internationalised name is refused rather than converted: a
document that means one writes its ASCII form (RFC 5890). A full URL grammar was not written
because the owner asked for rudimentary checks and a grammar is where a checker's own defects would
live. The RFCs are cited by section and paraphrased; none is quoted.

## 4. Tests

`a_malformed_host_is_refused_by_name_before_the_tls_stack`: an over-long name, a 64-octet label, a
space, a percent-encoded host, user information, ports `99999` and `+80`, an empty label and a
malformed IPv6 literal — each refused by the same named sentence at all four levels and by
`transmit` with the same `UrlRefusal`; a well-formed IPv6 literal, a 253-octet name with the root's
dot and a dotted IPv4 host pass. `a_plain_host_still_reaches_the_listener`: the loopback server is
reached through the check.
