# Session 1245 — the host is checked before TLS, and each hint-table run ends on a byte

Sequential mode, alone. Contract: the owner's answers `A130` and `A131` of 2026-09-23.

**A130 (ADR 1327).** `viewer_host::submit::check_url` runs in `transmit` after the scheme check and
in `may_submit` after its own, so a malformed host is refused at every level and never reaches
`rustls`. Checks: `//` authority present; no `@`, no `%`; a name of at most 253 octets (one root
dot admitted) with labels non-empty, at most 63 octets, of ASCII letters, digits and hyphens; or an
IPv6 literal that `Ipv6Addr` parses; a port of one to five digits naming a `u16` (`+80` refused);
no control or white space after the host. Each failure is a `UrlRefusal` whose sentence names it.
Not checked, on purpose: hyphen placement, fake IPv4, the path's grammar. `CLAUDE.md` principle 3
now names TLS for a transmission a person allowed as its one exception; `doc/stack.md` and
`doc/PLAN.md` say it is granted. `policy.rs` got the one call and a doc sentence (not in the
file list, needed for "at every level").

**A131 (ADR 1328, amending ADR 1293 section 4's packing bullet).** `hint_data` pads each item's run
across every entry to a byte in Tables F.4, F.6, F.8 and F.12; the test reader skips the same.
`every_item_run_of_a_hint_table_begins_on_a_byte` works out the run lengths from the header widths
and checks that the padded runs end exactly where the next table begins, with zero padding; it
fails when a single `align` is removed.

**Evidence (qpdf 12.4.1, not the derivation).** Fixtures: every shared-object length mismatch and
misread identifier is gone. What remains is qpdf's page-0-is-first reading and its object count per
page where thumbnails and beads sit in a page's section. Ten corpus files, classic tables:
freeculture, labelled_pages, tracemonkey, alphatrans, annotation-highlight, asciihexdecode, sizes
and issue1512r are clean; TAMReview warns only about its source's unsorted name tree; issue15590 is
exit 2 on `/Pages` naming a page, as its source is. With object streams, all ten warn "uncompressed
object after a compressed one in a cross-reference stream": that is F.3.1 numbering, not packing,
and it is left open.

**Rows.** F.4.1 stays `implemented`, and its note now gives the reading, the evidence and when to
revisit it. F.3.6's test list follows the renamed unit test. `doc/todo/57` and `doc/todo/65` no
longer list Q130/Q131 as open.

**Gates.** fmt, clippy (3 crates, `-D warnings`), nextest 857/857, conformance: exit 0. Behind the
lock: on_disk, transform gate, optimize_corpus (1918 linearised, 0 Annex F faults, 0 drew
differently), launch_path: each exit 0.
