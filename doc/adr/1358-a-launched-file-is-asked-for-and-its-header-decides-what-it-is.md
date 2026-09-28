# 1358 — A launched file is asked for, and its header decides what it is

Session 1260. Status: **accepted** and built.
Context: `crates/pdf-model/src/action.rs` (`launch`, `Action::Launch`, `RemoteAct`),
`crates/pdf-model/src/view.rs` (`perform`), `crates/viewer-core/src/command.rs`
(`Purpose::LaunchDocument`), `crates/viewer-core/src/interact.rs` (`open_remote`, `resume_remote`,
`states_a_pdf_header`, `place_remote`, `decline_remote`, `Locked::purpose`),
`crates/viewer-core/src/viewer.rs`, `crates/viewer-host/src/policy.rs`, `crates/viewer-ffi`
(`PurposeKind::LaunchDocument`, `QUORRA_PURPOSE_LAUNCH_DOCUMENT`), `crates/viewer-confined/src/protocol.rs`.
Builds: ADR 1368. Prices: the one departure of §12.6.4.6's ledger row.
Clauses: ISO 32000-2 §7.5.2, §12.6.4.3, §12.6.4.6 (Table 207).

## 1. Decisions

- **A launch with an `/F` is `Action::Launch`, carrying the `RemoteGoTo` shape** with `act:
  RemoteAct::Launch` and no destination. §12.6.4.6 says "[t]he F entry determines the file
  specification platform to be launched", and `/Win`, `/Mac`, `/Unix` are deprecated or undefined,
  so `/F` is the only entry read. An `/F` that §7.11 cannot read is refused by name.
- **A purpose of its own, `Purpose::LaunchDocument`**, rather than `RemoteDocument`. The act and the
  path rule are the same, but the question is not: a person is told that a PDF opens (beside, where
  `/NewWindow` is true) and that anything else starts nothing, under §12.6.4.6. The purpose crosses
  the confined wire as 5 and the C ABI as `QUORRA_PURPOSE_LAUNCH_DOCUMENT`.
- **§7.5.2's header decides after the bytes arrive**, in the 1024-byte window `pdf_syntax` measures
  from. A file specification's name is no evidence of type. A PDF is opened where opening puts it,
  since Table 207 names no page; the password hold of ADR 1335 applies unchanged.
- **Printed is read as the disjunction it is.** Only the deprecated `/Win`'s `/O` chooses printing,
  so the document is opened.

## 2. The departure, priced

A file that is not a PDF is an application, and it is not launched. The note names the file, says
its §7.5.2 header shows it is not a PDF, and says the sandbox withholds launching it. Principle 3
(ADR 0014) is the reason. Its cost is every document whose launch action names a program. The
corpus's one launch action is not one: `issue17846.pdf` names its file by Table 43's `/UF`, a path
into two subdirectories, which the path rule refuses at every level (ADR 1227). The drive shows the two halves on a built fixture: a PDF
beside the document opened in a new tab after the question, and a script named and refused.
