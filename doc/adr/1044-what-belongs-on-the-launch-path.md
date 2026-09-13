# 1044 — What belongs on the launch path, and where a document's own report went instead

Session 1027. Status: **accepted**. Takes `viewer_core::notes::about` off `Command::Open` and puts
it behind `Command::Report`, held by a `OnceCell` on `Open`. Adds a fifth row to
`doc/checks/launch-path.toml` — the first that is both large and signed — and moves two
`open_kinstructions` floors by the measured saving. **Does not** change one sentence any report
says, and does not change when a *page's* report is produced.

`§N` is ISO 32000-2 and nothing else.

## 1. The gate could not see the case it most needed to

`crates/viewer-ui/tests/launch_path.rs` bands five figures that principle 2 names, and it passed.
Its population was four documents — 5, 57, 1023 and 1 pages — and **not one of them was signed**.
A population is a claim about what a gate can observe (trap 8), and this one silently excluded the
most expensive thing an open did.

What it excluded: `notes::about` answers eight clauses about the *file*, and §12.8's answer makes
four passes over the bytes `/ByteRange` names — `Signature::excluded` and `Signature::signed_end`
walk them, `Signature::integrity` and `Signature::authenticity` digest them. On a signed document
that is most of the file, and `CLAUDE.md` principle 2 says plainly:

> Nothing eager. … Anything not needed to show page one is deferred until first use.

A signature decides nothing that page one draws.

**Observed before anything was fixed** (trap 13), with the row added to the gate first:
`doc/pdf.js/test/pdfs/xfa_filled_imm1344e.pdf`, three megabytes and one page, opened by reading
**1304 KiB in 118 read calls** and executing **47.6 M instructions**. The gate named all three as
outside their bands. After: **99 KiB, 26 calls, 1.80 M instructions** — a 96% cut in the
instructions an open executes, on a document whose page one needs none of the difference.

## 2. Where it went, and why not somewhere nearer

The report is produced by a **command a host sends**, `Command::Report`, answered with the
`Event::Reported` it always was. Three alternatives were rejected, each for a measurement rather
than a taste:

- **Later inside the open** — after the page events, still inside `Command::Open`. The figures do
  not move at all; the work is in the same command.
- **On the first `Command::RenderReady`.** The open figures come clean and `first_page_ms` keeps
  every millisecond of it, because the gate's first-page phase acknowledges the frame it drew. A
  fix that a gate cannot see is a gate quietly narrowed.
- **A `Query`.** `viewer-ffi` holds `tests/every_query_reaches_the_abi.rs`, so a new question costs
  an answer variant, a panel type and an ABI entry point; a new *command* costs a discriminant and
  reuses the event every host already handles. The vocabulary that was already there is the
  cheaper and the clearer one.

**When a host asks is the whole of the decision, and it is one rule in one place.**
`viewer_host::report::Due` is armed by `Event::Opened` and spent by the frame a host has just
presented — so the reader has page one before the file is digested, in the program as well as in
the gate. Asking earlier would move the same work back in front of the first page *while hiding it
from the instrument*, which is the worst of the four outcomes and the reason this paragraph is a
decision rather than a comment.

`crates/viewer-host/tests/every_host_that_arms_a_report_asks_for_it.rs` derives the host population
from the tree and fails if a crate arms the report without asking for it — calibrated by deleting
one call and watching it name `viewer-qt`. A report deferred into never being produced is a silent
regression of its own kind, and this is what stops it.

## 3. What the bands say now

The new row states `none` for its three clock figures: what it is *for* is the counted ones, which
no machine can move, and a clock band has to be derived on a quiet machine (`doc/todo/05`'s rule
for a figure whose band is not derived yet).

Two `open_kinstructions` floors moved, both by the saving measured under callgrind in both arms of
the same binary: `PDF20_AN001-BPC.pdf` 5089.8 → 4942.3 (floor 4996 → 4848) and `bug1815476.pdf`
3446.5 → 3394.4 (floor 3390 → 3337, which the saving had left 0.13% above a figure whose own spread
is 0.002%). **No ceiling moved and no band widened**: nothing about the expensive end of an open
changed, and a floor lowered by a measured win is what `doc/checks/launch-path.toml` asks a round to
do — session 925 took the same step on `turn_ms`. The other two rows keep both ends: their savings
were 0.33% and 0.19% against figures sitting 1.6% and 1.8% above their floors.

## 4. What this decides for later rounds

**The launch path is `Command::Open` and the first frame, and what may be on it is what page one
needs.** A claim about the *file* — a signature, a requirement, an attachment list, a version — is
answered when somebody asks, and `Open::about`'s `OnceCell` is why asking twice costs once. A round
that finds a new document-level sentence to say adds it to `notes::about` and pays nothing at
launch; a round that moves such work back into the open has to move a band, which is the argument
this ADR exists to make it have.
