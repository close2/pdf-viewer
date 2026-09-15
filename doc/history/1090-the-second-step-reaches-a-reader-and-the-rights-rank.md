# 1090 — §12.8.2's second step reaches a reader, and Table 258's rights rank beside Table 257's

Batch 1086–1091. Contract: §12.8.2's last two residues — no host asks any of it (ADR 1043), and §12.8.2.3's ranking
against Table 258's rights (round 1082). ADR 1104 is the argument.

**1. What a reader is told.** `viewer_core::notes::about` now says, beside the sentences it already said about each
signature, what the file did after it: the changed objects counted in Table 257's vocabulary, the verdict the
`/DocMDP` `/P` comes to, the verdict Table 258's rights come to, the objects a level does not permit or refuses to
rank, and the rights a `/UR3` grants that no changed object can be ranked against. It crosses the confinement and
the C ABI as the `Event::Reported` it always was — **no message, no entry point, `QUORRA_ABI_VERSION` unmoved** —
and is off the launch path because ADR 1044 put `notes::about` behind `Command::Report`: `launch_path` 26 banded, 0
outside.

**2. The rights ranking.** `Comparison::against_usage_rights` executes §12.8.2.3's second step — "modifications to
any objects that are not permitted by the transform parameters" — with Table 258's rights as those parameters. Each
changed object is classified a **second** time, into `Exercise`, in the same walk, because the tables disagree both
ways: a form field added is `Kind::Unclassified` and `/Form` `Add`; §12.8.4's validation material is carved out
there and refused here. `Operation` carries eight of the twenty-three rights, `RIGHTS_NOT_RECOGNISED` refuses the
other fifteen by name, and a change no right names is `Unrecognised` rather than `NotGranted` (ADR 1049 §2's
argument, other table).

**3. The census, 90 763 documents.** 327 state a `/UR3`: **199 changed nothing, 87 hold a change no right of Table
258 names, 6 are within the rights granted** (`prefilled_f1040.pdf` among them), 35 refuse comparison, **none is
outside** — because **all 327 state `/P` false or leave it absent**, which Table 258 makes the default and which
means "any possible restriction may be ignored". No fixture gives that. One spells a right `/Form ADD`, no Table 258
name.

**4. What the confined sweep caught.** `awkward_classes` killed five documents' `report` with `SIGSYS` the first
time it ran confined: `FileBytes::prefix` duplicated the file's handle — `fcntl(F_DUPFD_CLOEXEC)` — which the filter
kills rather than refuses (trap 31), so ADR 1043's `Err` branch could never have run there. `Held::OnDisk` now
carries the handle's own length beside the `Arc`: a prefix is the same open file under a shorter number, no call, no
descriptor, no byte — and the old shape would have spent the worker's eight-descriptor budget too. The test asserts
the raw descriptor number; `on_disk` holds the narrowed view to the corpus, object for object.

**5. Calibration** (trap 13) **and rows.** One fixture read twice, `/P 2` and `/P 3`, one update creating one
annotation — the level the only difference. The rights: a `/UR3` granting `/Annots [/Create]`, an update creating an
annotation (granted), deleting another (**outside**), deleting a page with its `/Pages` node (refused — Table 258
names no right for it, which is where the contract's page deletion lands); the control grants `Delete` too and moves
object 6 alone. §12.8.2, §12.8.2.2, §12.8.2.2.2, §12.8.2.3, §12.8.2.4: `partial` → **`implemented`**. Five siblings
are in flight here; what is red in the sequence is theirs, and the round's report carries every line with its code.
