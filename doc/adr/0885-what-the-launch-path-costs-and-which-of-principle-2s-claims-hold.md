# 0885 — What the launch path costs, and which of principle 2's claims hold

Session 922. Status: **accepted**. The second of this round's two records: what the instrument of
[ADR 0884](0884-the-launch-path-is-a-gate-and-a-wall-clock-band-needs-a-machine-under-it.md)
found when it was pointed at `CLAUDE.md` principle 2's own sentences.

## Context

Principle 2's *Startup time is a first-class requirement* makes five checkable claims about what
this program does when it opens a document. All five had been true when somebody wrote them and
none had an instrument. This round built the instrument (ADR 0884) and then read the claims off
it. The numbers below are not repeated in any instruction document — `tools/state.sh launch`
prints them — but the *readings* are here, because a reading is what no command produces.

## What holds

### "Nothing eager" — holds, with one sentence that does not

`strace -f -e trace=openat` over a child that does nothing but open the 1023-page specification
shows **eleven `openat` calls in the whole process, of which exactly one is not a shared library:
the document**. No configuration file, no recent-file list, no data resource. `Miniatures::new` is
`Default::default` and holds nothing until a panel asks. So *no configuration or recent-file
scanning* and *no thumbnail generation* hold, deterministically, and the gate now watches the
figure that would move if either changed.

**"No full page-tree walk" holds, and it is `/Count` that makes it hold.** `Pages::new` takes
§7.7.3.2's `/Count` where the root has children and a plausible value, and walks only where the
file has contradicted itself — so 1023 pages cost 0.254 ms against 5 pages' 0.037.

**"No system font enumeration" is false as a flat sentence, and true as principle 2 means it.**
A page that names a font it does not embed sends `pdf_font::substitute` through the machine's font
directories on the launch path — ADR 0870 is where that first bit, in the confinement — and it is
measurable from outside: `doc/pdf.js/test/pdfs/bug1815476.pdf` opens **23 files under
`share/fonts`**, does **47 more directory listings** than a document whose fonts are its own,
reads 2.5 MB for a 131 kB file, and takes **69.7 ms to its first page against 39.4** for a
five-page document with embedded fonts. That is not eager by the principle's own definition —
"[a]nything **not needed to show page one** is deferred until first use", and this is needed to
show page one — but the bullet above it says *No system font enumeration* without a condition, and
a reader checking the sentence against the program would find it false. The gate now carries that
document as its fourth row, so the cost of that path has a band on it rather than a sentence.

### "No parsed data at startup" — holds

`pdf-spec`'s build script emits **612 `static` key tables** into `OUT_DIR` from the Arlington
model — one `KEYS_*` per object the model describes — so
the object model is `.rodata` and costs no parse at launch; the `strace` above is the other half of
the proof, since a resource parsed at startup is a resource opened at startup. Everything else on
the path that could be eager is a `OnceLock` — `pdf_syntax`'s whole-file fallback, `pdf_model`'s
press table, `pdf_font`'s predefined CMaps — which is the construction the principle asks for by
name.

### "Page one goes to the GPU, and nothing waits for warmth" — holds, and the trace says so itself

Under `Xvfb`, `--trace` prints `pipelines compiled in 11.619771ms, noticed at this frame — nothing
on the launch path waited for them`. Cold bring-up is now a gate of its own: **27 to 30 ms on this
machine's real adapter**, headless, named in the run's own output (`AMD Radeon 890M Graphics (RADV
STRIX1) (IntegratedGpu, Vulkan)`), which is the first time this project has measured a launch on
the adapter rather than on `lavapipe`.

### "Incremental parsing … not the whole file" — holds, and it is session 881's doing

Opening the 19 MB specification reads **4325 KiB of 18 756**, counted as `rchar` from
`/proc/self/io` across the open alone. Before ADR 0809 put `FileBytes::on_disk` under the hosts
this was the whole file, every byte, and the claim was true of the parsing and false of the bytes
— which is the one of these five claims that *changed* rather than being confirmed. What the
remaining 23% is composed of is not attributed here: the whole open is 13.2 ms warm, of which
`Document::open` is 3.9 and §12.3.3's outline 2.7 on 988 items, and nobody has yet asked which of
those the bytes belong to.

## What does not hold

### "A 500-page document must open no slower than a 5-page one"

**False of the open, by a factor of thirty, and true of the launch.** Both halves matter and the
sentence states only the first:

| | 5 pages | 1023 pages | ratio |
|---|---|---|---|
| cold open (page cache dropped) | 0.61 ms | 23.28 ms | 38× |
| warm open | 0.42 ms | 13.16 ms | 31× |
| `Document::open` alone (`examples/open_cost`) | 0.106 ms | 3.931 ms | 37× |
| **time to first page** | **39.4 ms** | **43.4 ms** | **1.1×** |
| what the open costs in memory | 7 MiB | 18 MiB | 2.6× |

The launch is equal because the document is not on its critical path: in **every** run of the
gate, `document joined` and `device up` are the same figure to a tenth of a millisecond — the
document thread finishes before the graphics device does, so a document thirty times more
expensive to open is still free. `doc/performance.md` has said "1023 pages and 5 pages now cost
the launch the same" since session 289 and `doc/todo/42` has kept the question open as a question
about the function; what is new is that both halves are now measured by one command, and that the
margin is legible — the open would have to grow by another 20 ms before it began to show.

This is a claim about principle 2's *wording*, and this round does not touch it: round 921 has
already filed the wording of that section as a question for the owner, and these numbers belong
beside it.

## The rest of what was measured

The whole launch **with a window**, which the gate deliberately does not cover, re-taken this
round under `Xvfb` on the software adapter for the 1023-page specification: **120.7 ms to the
first present** — arguments 0.3, chrome fonts 3.1, event loop 30.4, window 30.8, graphics instance
33.3, graphics device 47.3, surface configured 69.4, document joined 69.5, page one interpreted
(547 commands) 81.1, first scene built 84.5, first present 120.7 — with five arrow keys presenting
in 8.8 to 14.4 ms. That is the same shape `doc/performance.md` records from sessions 292 to 445,
one round of instrumentation later.

**And the memory high-water of this program is the graphics driver, not the PDF.** Opening the
largest document this project owns costs 18 MiB; a process that has brought a device up and drawn
one page costs 171 to 178, and 190 where a substitute font was loaded. Any future work on the
memory gate should start there rather than in the reader.

## Consequences

- Four of principle 2's five startup claims hold, one holds only in the sense the principle means
  and not in the words it uses, and one is false as written and true of the thing it is about.
  Every one of them now has a number and a command that prints it.
- The claim that changed under session 881 is the incremental-parsing one, and it changed in the
  right direction: a claim that used to be true of the parse is now true of the bytes.
- The `read_kib` and `open_peak_mib` bands in `doc/checks/launch-path.toml` are what keep the
  three deterministic claims from decaying quietly. The font finding is a row rather than a
  sentence for the same reason.
- Nothing in `CLAUDE.md` was amended by this round, deliberately.
