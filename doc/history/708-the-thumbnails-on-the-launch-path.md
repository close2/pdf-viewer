# 708 — The thumbnails on the launch path, and the round that killed its neighbours

Sixth merge round of the block. Four branches, **no conflicts**, and two findings only a merge round
could make: one that ties three rounds' unexplained failures to a single cause, and one that this
round's own predecessors had each got half of.

## The sequence, whole, on a quiet machine (load 1.29)

Both workers built first. `fmt` · `clippy --workspace --all-targets` under `-D warnings`, exit 0 ·
the fuzz check, exit 0 · `nextest` **2528 passed, 18 skipped** · conformance 182 + 5 + 1 + 1 ·
`cargo deny` all four ok · corpus **974 documents, 67 incomplete** · oracle **1945 pages — 983
agrees, 65 contradicted, 832 ambiguous, 42 not comparable** · `render-quorra` **932 agree, 23
differ** · accessibility census **1336** · `fixed_documents` 40/0 · text, selection, dates, XMP,
JPEG 2000. §5's binaries rebuilt and installed.

Ledger **445 implemented, 223 partial, 17 reported**, 0 unreviewed — 706's §11.7.5.2 moving
`reported` → `partial`, which is the one status that went *down*.

## The launch-path violation, in the host that was ahead

704's headline is a `CLAUDE.md` §2 breach of the kind that file forbids **by name**: *"no thumbnail
generation on the launch path"*. `viewer-ui` built the whole of §12.3.4's list at tab-open, and Table
29's `/PageMode /UseThumbs` opens that tab **as the document opens**.

| | first present | §12.3.4's list |
|---|---|---|
| `viewer-ui` before | 156 ms | 1000 rows, **121 ms** |
| `viewer-ui` after | **48 ms** | 8 rows, 0.30 ms |
| `viewer-gtk` | 108 ms | 1000 rows, nothing decoded |
| `viewer-qt` | 74 ms | 1000 rows, nothing decoded |

**First present 3.25× faster** on that document. The briefing had said `viewer-ui` was ahead on these
three panels; it was ahead on *having* them and behind on obeying the clause about them, and **the
eager build is what the item actually cost**.

The parity shape held for a third item: `viewer_host::Tab` is a closed list of six panels carrying
Table 29's mapping, matched exhaustively in all three hosts, so a seventh fails to compile in three
places — with each host also carrying a runtime test tying its toolkit's ordering to the shared list.
Tenth host item running that needed **no new message**.

## The `pkill` that killed its neighbours — three rounds, one cause, and only a merge round could see it

Every parallel worktree lives under a path containing this project's name. So a round running
`pkill -f pdf-viewer` to clean up its own windows matches **its own shell and its siblings'**,
because every command line they are running contains that string by virtue of their working
directory.

- **704** lost four commands to it before seeing the pattern.
- **705** saw three `cargo build --release` invocations return **exit 144** with every artifact
  produced correctly and freshly dated, and recorded it as a harness artefact under heavy load —
  *reasonable, and wrong*.
- **707**'s two wait-loops died the same way.

Three rounds, three separate write-ups, one cause. The rule is now in `doc/environment.md` beside the
stash and the scratchpad, because it is the same shape — **a namespace the machine gives every round
by one name is a namespace two rounds will collide in** — and it is worse than either, because **the
victim is a sibling, so the round that pays is not the round that erred.** `pkill -x`, and *if a
command returns 144 with its output intact, suspect a neighbour before suspecting the harness.*

## 706 — the sixth price of this block, killed by a clause's grammar

`doc/todo/13`'s remainder was priced as a per-pixel transfer channel plus a matching pass in all
three backends. **Half of it needs no pixel at all**, and the deduction is three lines: take an
object §11.7.5.2 does not call fully opaque and any point it encloses — either it is topmost there
and the first sentence withholds its function in favour of the default, or something else is topmost
and *that* object's function or the default is chosen. Neither branch can ever choose it, so its
function is used **nowhere on the page**. One object, one graphics state.

**The tell is worth more than the item**: the clause's own sentence separates the two questions and
the price did not. Its subject is a *point*; the qualifier that makes half of it cheap — "but only if
the object is fully opaque" — attaches to the *object*.

`TransferState::in_force` is **gone**, and all five of its callers turned out to be *answering the
clause by not asking it* — trap 2's shape one layer down. And the same deduction one step earlier
found a real defect: a soft mask's luminosity was computed from **mapped** colours, silently, where
§11.5.3 performs that conversion "with no compensation for gamma or other colour calibration".

It also found Annex N, which defines a nearly opposite object-based model for this parameter,
informative and gated by §N.1 on a halftoning device — *worth knowing before somebody finds N.3 and
reads it as a permission* — and corrected a ledger row that has named the wrong field since session
637, harmless while the answer reached only a report and now deciding a colour.

## 705 — a criterion turned into a measurement, and a paragraph in fifteen rows

701's criterion was *a claim held in duplicate has somewhere to disagree with itself*. 705 made it
computable: for every parent whose subtree holds ≥2 `partial` rows, count the rare five-word
sequences the notes share pairwise. §12.8 heads the ranking. **Deliberately not built as a sweep** —
its output is a ranking rather than a hit list — and the recipe went into `doc/todo/01`.

What it found justifies it: **one 92-word paragraph standing byte-for-byte identical in fifteen
rows**, ending "eight signed documents, twenty-six encrypted", where the measured figures are **nine**
and **twenty-five** — and §7.6's row has said twenty-five since 691. Beside it, §12.8.3.4 and
§12.8.3.4.5 still answered the signature-value question **for RSA and DSA only**, four rounds after
ECDSA and EdDSA landed and while four sibling rows recorded all four; and `authenticity`'s own doc
comment listed three of five constructions **twelve lines under a module comment saying "for all
four"**.

**It read §7.6.4.4 — still rank 1 by blame — and left it alone**, its arithmetic holding against all
twelve rows below. A row left uncorrected beside four the round rewrote, *checked* rather than
assumed, is worth recording as work.

And it declined to repeat an ordinal of this project's own reporting: the "eighth consecutive round
the errata check has paid" could not be verified against a command, so it wrote the **property**
rather than the count. That is ADR 0281's rule applied to a sentence about the rounds themselves.

## 707 — two silences closed, and a reference that was wrong

**The census names a cause now.** The interpreter attributes an `Unsupported` to the marked-content
sequence enclosing it, so the census prints how many elements enclose a refusal and, per page with
that page's own report sentence, how many have both no place and a refusal inside them. **Trap 11
decided the condition and rejected the reflex one**: *placeless on a page that reported* fires on a
condition §14.8.3.3 does not state — the clause states **enclosure** — and a test pins the difference
with two elements that enclose an undefined `/XObject` and still have a place because they also drew
text.

Its calibration closes 700's finding exactly: one binary, one variable. With the worker, 2 enclosing
and 0 placeless-and-refused; without, 11 and **9** — accounting for the whole of the −9 that had been
moving invisibly.

**Two harness defects behind four of the six uninformative oracle lines.** `mutool draw` creates its
output file *before* deciding it cannot draw, so a zero-byte PNG passed `exists()` and became a
harness error — **the one failure the cache refuses to remember, so those pages re-ran `mutool` on
every run** — and `gs` writes its diagnosis to **stdout**, which the harness sent to `/dev/null`.

**One of the six is the reference being wrong, proved without reading our code.** `pr6531_2.pdf`'s
empty password authenticates against its `/O` under §7.6.4.4.11's Algorithm 12, computed by hand from
the file's own bytes — so poppler, ghostscript and this tree are right and `mupdf` 1.28 accepts only
the user password. The other five lost a reading because the *document* is outside what the standard
describes, one of them a JP2 on which poppler's OpenJPEG dumps core.

**And the specification question got a real answer: two references stay enough.** ADR 0005's
inference is about a *pair*; a third multiplies the improbability rather than creating it. What the
six are actually about is a different precondition — the surviving pair partly agrees about
**repair**, which no clause states.

One general fact recorded: **a cache's key does not include the harness's prose**, so 92 cached
entries still carry the old wording.

## Owed

- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **`render-quorra`'s corpus** is the one gate not measured without the worker — it costs a device run.
- **Nothing counts what a *window* cannot reach** the way `tools/state.sh hosts` counts what a C
  caller cannot; §12.3.5's collection and §12.5.6.14's popups are `viewer-ui`'s alone and are not tabs.
- **92 cached reference failures carry the old wording** until something rewrites them.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
