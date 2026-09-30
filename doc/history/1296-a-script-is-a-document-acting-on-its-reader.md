# 1296 — A script is a document acting on its reader: RFC 0008

Date: 2026-09-30. Branch: `batch-1290-1295`, shared with round 1293. No ADR: an RFC is the
deliverable and the owner decides. Commissioned by the owner: *"use one round to create an RFC for
adding JavaScript. what would it mean, where should we draw the line, which library..."*

## What was built

**RFC 0008** (`doc/rfc/0008-a-script-is-a-document-acting-on-its-reader.md`): every site the
standard names, quoted; what the tree does with each today; a census; three tiers with a reason
each and a named failure for every excluded call; six engines with measured timings; the
architecture (a `pdf-script` crate, a third confined worker, the edit log, the event order, a
`Scripts` level defaulting to `off`, the gates, the failure modes); the ledger both ways; the
costs; the five current restrictions with their rationale; ten questions; what is built when.

**The census** (`crates/pdf-model/examples/javascript_census.rs`): per document, whether it carries
a script, at which site, and — by a tokeniser, never an engine — which names it calls, against the
RFC's tiers. The first walk died at the data limit on a six-gigabyte Tika artefact; a file over
128 MiB is now counted per corpus instead of read. 929 of 90 319 carry a script; the commonest
script in the world is Adobe's own viewer-version check (`this.ADBE`, `app.findComponent`).

**The timing probe** (`js-timing.txt` beside the RFC): Boa constructs in 0.4 ms and has no memory
ceiling; QuickJS in 0.3 ms with a working interrupt and memory limit; Duktape on an unmaintained
binding; V8 a 64.7 MB binary failing `cargo deny`. Boa recommended, confined; QuickJS the alternative.

## Three things worth keeping

**The brief's table numbers were wrong and the text was read instead**: the script action is
Table 221 (not 217), the additional-actions tables 197–200 (not 195–199); `doc/todo/56` §4.1's
`partial` rows are `implemented` now.

**Adobe's reference moved in a month** — the URL `doc/todo/56` pinned answers 404; the content is
in `adobe/dc-acrobat-sdk-docs` at `ab3b42a7`. **Adobe's privileged marker is not this project's
line**: `app.openDoc` and `this.submitForm` are unprivileged there; here the line is the process.

## Files touched

`doc/rfc/0008-a-script-is-a-document-acting-on-its-reader.md` and `…/js-timing.txt` (new),
`doc/rfc/README.md` (one row), `doc/questions/Q193-*.md` (new), `doc/todo/56-*.md` (five targeted
edits), `crates/pdf-model/examples/javascript_census.rs` (new), this file.
