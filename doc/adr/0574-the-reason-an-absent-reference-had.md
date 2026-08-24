# 0574 — The reason an absent reference had, on the stream nobody read

Status: accepted, in the seven-hundred-and-seventh session.
Supersedes nothing; it continues ADR 0542, which made a missing reading *visible* and left its
*reason* to the harness.
Touches `tools/pdfref/src/reference.rs` and one new test in `tools/pdfref/tests/end_to_end.rs`.
**No verdict moved**: 983 / 65 / 832 / 3 / 2 / 42 / 18 before and after, on two runs of the same
tree.

## What ADR 0542 left

The six-hundred-and-ninety-fourth session found that `render_references` discarded a failed
reference silently whenever two remained, so a page judged on two printed a line indistinguishable
from one judged on three. It fixed that: the absence is now on the page's own line as
`[judged without: <reference> did not render: …]`, and counted in the summary. Six corpus pages
print it.

**What the six actually said was the harness's sentence and not the renderer's.** Four of them read

```text
mupdf did not render: PNG error at …/mupdf.png: unexpected end of file
```

which names a PNG decoder and no document. Two mechanisms produced that, and each is trap 3 one
step further in than the invocation it is usually about.

## The first: an empty file passed the test for a file

`Reference::render_within` judged success by `output_path.exists()`, on the sound reasoning that
"Ghostscript and mutool both report real problems on stderr while still exiting zero, so success is
judged by whether an image appeared". `mutool draw` **creates its `-o` file before it decides it
cannot draw the page**, so a document whose page tree it cannot recover leaves a zero-byte PNG
behind. That existed, so it reached `png_io::read`, so the failure came back as
`HarnessError::Png`.

Two things followed from the wrong variant, and the second is the expensive one:

- the gate printed the decoder's sentence where the renderer's own was in the log beside it;
- **`cache::write_entry` declines to remember a `Png` error** — correctly, and its comment says why:
  "a PNG this harness cannot read … is not a property of the document, and remembering either would
  outlive its cause". So those pages re-ran `mutool` on every oracle run, for ever, in a cache that
  is otherwise at a 99.8% hit rate.

A renderer that produced no bytes has produced no output. The condition is now `!exists() ||
len() == 0`, which puts the failure in the variant that is a property of the document and is
therefore remembered.

Where there *are* bytes and they are not a PNG this harness can read, the variant is deliberately
left as `Png` and the renderer's own line is added to its message: a half-written image is the one
failure here that can be the machine's rather than the document's, and it must stay unremembered.

## The second: the log held the consequence and not the cause

`last_line` took the last non-empty line of the renderer's log, "because these renderers narrate
their progress and warn about recoverable damage, so what finally stopped them is at the end". That
is true of the *stopping* and false of the *reason*, and all three references prove it on this
corpus:

| | first line | last line |
|---|---|---|
| `mutool` | `argument error: invalid page number: -1` | `cannot draw '<path>'` |
| `gs` | `Error: /undefined in obj` | `GPL Ghostscript 10.07.1: Unrecoverable error, exit code 1` |
| `pdftoppm` | `Syntax Error: Malformed JP2 file format: first box must be JPEG 2000 signature box` | the OpenJPEG assertion that killed it |

Each pair is one sentence naming what the renderer met and one naming nothing much, and the gate
was printing the second. `diagnosis` prints both, joined by `…`, and only where they differ. Two
rather than the whole file because this string goes on a gate's own line and into a remembered
failure; the log is on disk beside the render for anyone who wants the middle.

## The third, found while checking the second: `gs` speaks on stdout, and the harness sent it to `/dev/null`

The command was built with `.stdout(Stdio::null()).stderr(log)`. Ghostscript writes `Error:
/undefined in obj` and its operand stack to **stdout**, and only `Unrecoverable error, exit code 1`
to stderr — so on `bug1606566.pdf`, a file with no `%PDF–n.m` header at all (§7.5.2), the gate could
print nothing about why. `Reference::version` has known which stream `gs` speaks on since it was
written; nothing had joined that to this.

Both streams now go to one log through a single file description (`try_clone`, so the renderer's own
interleaving survives; two opens at offset zero would overwrite each other). No renderer here writes
its image to stdout — all three are given an output path — and a healthy `gs` run writes **zero
bytes** there, measured, so this costs nothing on the pages that work.

## What the six lines say now

```text
GHOSTSCRIPT-698804-1-fuzzed.pdf p1  mupdf … argument error: invalid page number: -1 … cannot draw
bug1606566.pdf p1                   ghostscript … Error: /undefined in obj … Unrecoverable error
bug_jpx.pdf p1                      poppler … Malformed JP2 file format: first box must be JPEG
                                    2000 signature box … opj_int_ceildiv: Assertion `b' failed.
issue18986.pdf p1                   mupdf … argument error: invalid page number: -1 … cannot draw
issue21436.pdf p1                   mupdf … argument error: invalid page number: -1 … cannot draw
pr6531_2.pdf p1                     mupdf … argument error: cannot authenticate password … cannot draw
```

ADR 0575 is what those six turn out to be.

## Calibration

`end_to_end.rs::a_renderer_that_writes_an_empty_file_has_produced_no_output` writes a file that is
not a PDF, asks `mutool` for page one, and asserts both halves: the variant is `RendererFailed`,
which is what makes the failure remembered, and the message carries the renderer's own **first**
line. Run against this file before the change it fails on both.

## One thing this does not fix, and it is worth writing down

**A remembered failure carries the wording of the run that stored it.** `cache`'s key is built from
the format tag, the renderer's version, the document's digest, the page, the resolution and the
invocation — deliberately, and the module's own comment argues for every one of them — and the
*harness's* wording is in none of it. So changing how a failure is worded leaves every stored `.err`
entry saying what the previous version said, until it is regenerated. It is a message rather than a
verdict, and bumping `FORMAT` to flush it would invalidate 28 648 stored renders to correct 92
sentences. The entries this round could reach were cleared; the shared cache's were not, and the
wording there heals whenever an entry is rewritten. Same family as trap 10a: a cache's key is a
claim about what makes two answers the same answer.
