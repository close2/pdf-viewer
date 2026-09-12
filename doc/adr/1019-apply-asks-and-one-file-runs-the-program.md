# 1019 — `apply` asks, and one file runs the program

Status: accepted. Session 999.
Context: `crates/pdf-transform/src/{tool,executor}.rs`,
`crates/pdf-transform/src/archive/{config,remedies,decision,rewrite,prepare,report,mod}.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`, `crates/pdf-transform/tests/{archive,profiles}.rs`,
`doc/profiles/{derive-attachments,keep-everything}.toml`, `doc/rfc/0007` sections 2.1, 4.1–4.5 and
5b.1, `doc/questions/A54`, `A55`, `A56`, `doc/pdf-a-mitigations.md` section 0.2, RFC 0002 sections 5
and 9. Companion to ADR 1012 (the configuration format) and ADR 1018 (the departure).

## The owner's three answers, and what each one fixed in the code

`doc/rfc/0007` was accepted on 2026-09-12 with five questions answered. Three of them decide this
round, and each is a shape rather than a feature:

- **`A54`** — `apply` does **not** run the tool. It returns a *request*: a data value naming the
  program, the arguments, the input, the expected media type and the bounds. The caller executes
  it, and the caller is this project's own converter program, so a user types one command and the
  remedies happen. **One shared executor** in every consumer we ship, so the only code in the tree
  that spawns a process is that one file.
- **`A55`** — `derive` is offered and made impossible to get by accident.
- **`A56`** — no confinement offer in the first version; the warning lives where an operator
  configures a tool.

## The request type, and why two passes rather than a callback

A callback would have been shorter: hand `apply` a closure and let it call out when it needs a
program. It is rejected for the property RFC 0002 section 5 is built on. A closure is not data — it
cannot be written to a file, compared, or replayed — and section 9's determinism claim would then
rest on whatever the closure happened to do rather than on the conversion. So the seam is two
passes over one plan:

1. `apply` builds a [`ToolRequest`] per place a configured `derive` site failed, from the document,
   and returns them in `Report::requested`. The requirement is refused for that pass with the fifth
   kind of *no* — `Because::AwaitingTool` — and **nothing is written**.
2. The caller runs each request through `executor::execute`, puts the results in
   `ArchivePlan::tool_outputs`, and applies again. The second pass is a pure function of
   `(sources, plan, policy, budget)` exactly as before, because a recorded result *is* part of the
   plan.

`quorra-transform archive` does both passes in one command, which is the whole of the owner's
"a normal user would expect it just to happen". The loop is nine lines in `run()`.

The request's identifier is `<requirement>/<object number>/<generation>`, taken from the source's
own numbering, so the second pass finds each result where it looks for it and a recorded output
stays valid as long as its fixture does.

## Where the executor lives, and why not `viewer-core`

`cargo metadata` settles it rather than taste. The three consumers `A54` names reach the converter
like this:

| consumer | crate | how it reaches `apply` |
|---|---|---|
| the command-line program | `pdf-transform` (its own `src/bin/`) | directly |
| the KIO worker | `pdf-vfs-ffi` → `pdf-vfs` | `pdf_vfs`'s commit path |
| the FUSE filesystem | `pdf-fuse` → `pdf-vfs` | the same |

`pdf-vfs` depends on `pdf-transform`; **`viewer-core` is depended on by none of the three** (it is
`pdf-transform`'s dependency, for `Secret`, and nothing more). So `pdf-transform` is the one crate
every shipped consumer already has, and the executor is `crates/pdf-transform/src/executor.rs`. Its
one public function is `execute(&ToolRequest) -> Result<ToolResult, ExecuteError>`.

That it sits in the same crate as `apply` is not a contradiction of the purity rule: the rule is
about what `apply` does, and `apply` never calls this module. Putting it a crate away would have
made the rule *look* enforced while costing every consumer a dependency it does not otherwise need.

**The KIO and FUSE faces owe their loop**, and it is one loop rather than two: both hold the
converter through `pdf-vfs`, so `crates/pdf-vfs`'s commit path is where it goes, beside its own
`pdf_transform::apply` call. `A54`'s `Owes:` line is narrowed to exactly that.

## Three descriptors are files, and that is the design

A child whose output is a pipe cannot be waited on under a timeout without somebody draining the
pipe, and a parent that waits before draining deadlocks the moment the child fills it. The executor
hands the child *files* for standard input, output and error, in a directory it made:

- the document's bytes are written to a file and the program is given that file — never the
  document's own name, and never anything document-derived in `args` (section 4.1);
- the wait loop does no I/O at all, so the timeout is a poll and a kill rather than a thread dance;
- the result is bounded by asking the file its length **before** a byte is read, so a program that
  writes without stopping is not this process's memory problem;
- the directory is a `Drop` guard, so every early return removes it.

`{in}` and `{out}` are substituted here and nowhere else, because only the executor has a
directory; `{site}` and `{target}` are substituted where the request is built, both being the
configuration's and the caller's own words.

**`{media-type}` is refused by name, and that is a departure from section 4.1's list.** The RFC
names it a placeholder, and at the one site this version runs a tool for, the only media type
available is the attachment's own `/Subtype` — which comes from the document, and section 4.1's
other rule keeps document-derived strings out of `args`. Two rules of one subclause disagree at
this site; a configuration naming it is an error saying so, rather than a placeholder quietly
filled from a place the RFC forbids.

`{out}` is a **directory** rather than a file path, and the result is the single file the program
leaves in it. That is what `soffice --outdir {out} {in}` needs; zero files and more than one are
both failures named rather than guessed at. A tool given no `{out}` writes on standard output.

## Replay, and the one byte range that is not a function of the inputs

`tests/archive.rs::a_replayed_tool_output_converts_to_the_same_bytes` records one tool output,
**deletes the program**, and converts twice. Both runs produce a conforming file and the same
bytes. The exception is the `stEvt:when` of the recorded `xmpMM:History` action, which ISO 19005-2
section 6.6.6 asks for and which is a fact about when the action happened; the test blanks those
and says so, exactly as `the_same_input_twice_produces_identical_bytes` handles the same clock by
choosing a fixture that records nothing. So RFC 0002 section 9's claim is narrowed in writing to
what is true: **a conversion that invokes no tool is deterministic outright, and one that invokes a
tool is deterministic given its recorded output.**

## `derive`, and the four terms `A55` sold it on

Built at two requirement identifiers — `embedded-files/embedded-file-is-itself-pdfa` and its plain
PDF/A-4 sibling — which are one rule under two spellings and the catalogue's flagship entry. A
declared program is handed the attachment's bytes and the PDF it returns replaces them. The four
terms are code rather than counsel:

1. **Never a default.** A site absent from a configuration is `stop`; a site present without a
   `remedy` word is `stop`.
2. **Never reachable without the site *and* the tool.** `ConfigError::DeriveWithoutTool` refuses a
   `derive` row naming no `tool`, by site and by line — and `Derivation` cannot be constructed
   without a `Tool`, so the guarantee survives a future caller that builds a plan by hand.
3. **Reported per document in those words.** `Decision::Configured` carries
   `"this is derived, not original"`, and `Conversion::derived` names the attachment, the media
   type it was, the tool, the resolved program, the byte count and the SHA-256 of what came back.
4. **Recorded in `xmpMM:History`** — and the record is a *condition*: a document whose packet will
   not take the entry does not get the remedy either, which is `A48`'s construction applied to
   `A55`'s. The permission and its condition are one thing.

Two smaller decisions, both stated rather than taken quietly:

- **`on-failure = "discard"` is a configuration error at a built `derive` site.** Dropping an
  attachment is a rewrite of the embedded-file name tree nobody has written, and offering it here
  would be selling a permanent hole in somebody's archive to get past an afternoon of ours
  (`doc/pdf-a-mitigations.md` section 0.2's own rule, one level up).
- **The attachment keeps the producer's `/F` and `/UF`.** A derived attachment may now be a PDF
  under a name ending `.csv`, which is misleading; renaming it would be this converter inventing a
  file name, which `A48` puts on the far side of the line. The report and the packet say it is
  derived, which is the honest version of the same information. `/Params` `/Size` is restated
  (§7.11.4.1's Table 45 makes it the uncompressed file's size) and `/CheckSum` is removed rather than
  recomputed — it was a digest of bytes that are not in the file any more, and writing ours where
  the producer's stood would be asserting something about a file nobody has.

## `supply`, the fifth kind, built at one of its twelve

`doc/pdf-a-mitigations.md` section 0.2's finding. `embedded-files/associated-file-media-type` is
the entry the catalogue calls the best test of the owner's question: ISO 19005-4 section 6.9, by
way of ISO 32000-2 §14.13.2, asks an associated file's stream for a `/Subtype` that is a MIME media
type, and nothing in a file specification states one — an extension is a convention rather than a
declaration, so reading it as one would be this converter asserting what the bytes are. The
operator whose pipeline made the attachments knows.

`supply` carries an obligation the other four kinds do not, because it is the one kind the
converter cannot get wrong and the person can: the report states the supplied value beside the
requirement it answered, and `xmpMM:History` records that a human rather than the document is its
source. Both are built, and both are conditions in the same sense `derive`'s record is.

`unlisted` takes only `stop`: an attachment whose extension the operator's own table does not name
leaves its requirement refused, because guessing is the act the remedy exists to avoid. An **empty**
table is not an error but a stop — `doc/profiles/keep-everything.toml` already wrote `role-map = { }`
with the note *an empty map stops*, and a half-written profile should stay loadable.

## The warning (`A56`), and where it actually prints

Three places, all of them where an operator is looking at a tool:

- **`archive --remedy-sites --to <target>`** prints it under every site that takes one, and once
  more at the end of the listing;
- **every shipped profile that declares a `[tool.…]` block** carries it, and
  `tests/profiles.rs::every_profile_declaring_a_tool_carries_the_warning_the_owner_asked_for` is
  what keeps a future profile from growing a tool without it;
- **the run itself** says it on stderr, once per declared tool when the configuration is read and
  once per invocation before the program starts.

The sentence is `archive::UNTRUSTED_INPUT_WARNING`, one constant, so the three cannot drift:
*this runs a program you chose, on a document you did not write.*

No confinement is offered, which is `A56`'s choice and not an omission: this project confines its
own renderer because it can, and cannot hold somebody else's program to that standard; a profile
written against no particular program is a guess, and offering one for the few tools that would
run under it invites the belief that it is offered for all. `executor.rs` is where a per-tool
confinement would go the day a real deployment brings its own program.

## The stand-in, said plainly

Nothing on this machine turns a spreadsheet into a PDF/A, so the program exercised end to end is a
shell script the test writes: it reads the attachment on standard input and writes the PDF that
attachment becomes. What that proves is the whole seam — the request is built from the document,
the caller runs it, the bytes come back, the promised media type is checked, the attachment in the
output is the tool's bytes, the output is held to the target again, and the report and the packet
both say what happened. What it does not prove is that `soffice` works, which is not this project's
to prove. `doc/profiles/derive-attachments.toml` is the declaration a real deployment edits.
