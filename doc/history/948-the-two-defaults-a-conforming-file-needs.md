# 948 — The two defaults a conforming file needs, and a fifth kind of decision

Date: 2026-09-10. ADR: 0948.
Files: `crates/pdf-transform/src/archive.rs`, `crates/pdf-model/src/xmp.rs`, `data/icc/`.

The output intent and the metadata, which between them were what most of the converter's refusals
were waiting on. Conversions rose from 57 to 200 across the two targets then exercised.

**`A18`'s condition did not fit any of the four decision classes, and that is the finding.** An
output intent loses nothing, so it is not one of §3's authorised losses; but it changes what every
device colour in the file *means* to a conforming reader, so calling it `Mechanical` — a rewrite
that loses nothing — would have been true and misleading. `Decision::Stated { rewrite,
reinterprets }` is the fifth variant, and **the sentence travels inside the decision** rather than
being left to whoever prints the report. That makes ADR 0927's shared condition structural instead
of a caller's duty, and `A21`, `A48` and `A50` are the same class arriving later.

Two clause readings worth keeping. PDF/A-4's page-level output intent is *not* needed, because
ISO 19005-4 section 6.2.3's page rule binds only where the document states none. And where a file
already holds a destination profile the new entry **shares that object**, because the same clause
requires every entry of the array to name the same one.

**The XMP writer edits by span, and a writer built on this tree's reader would have been silently
lossy** — that reader keeps no `rdf:about` subject and no qualifier but `xml:lang`, so
round-tripping a producer's packet through it would drop whatever it does not model. A packet that
cannot be edited is refused by name rather than replaced.

Established right in three ways that do not depend on each other: the identification properties
are written to satisfy `pdf-archive`'s own metadata rows, which is the tightest specification of
correct available here; stage three re-opens every output; and the corpus run holds every
conversion to its target twice.
