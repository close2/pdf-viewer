# 0842 — A file made smaller by four passes, each derived from a clause, and none of them touching a mark

Session 900. Status: **accepted**. The fourteenth decision record of RFC 0002's implementation,
on the long-lived branch `round-867`, and the suite's seventh verb on ADR 0817's serializer —
the one that pays §7.5.7's producer debt that record opened.

## Context

RFC 0002 section 6.5 proposes `optimize` and surveys two schools. **Ghostscript re-distils**:
`-sDEVICE=pdfwrite` interprets a document down to marks and writes a new one, so the output is
appearance-preserving and structurally the distiller's — tagging, form structure and every
object it did not understand are gone. **qpdf preserves structure**: object streams,
cross-reference streams, recompression, dead-object removal, each a decision about the *file*.

`CLAUDE.md`'s amended exclusion decides between them before any argument about quality does.
Its boundary is "does the operation invent marks?", and its sentence about this suite is that
"every content stream in their output is a producer's, carried byte for byte **or recompressed
without reinterpretation**". A re-distiller's content streams are its own, so the second school
is not merely preferred here, it is the only one in scope.

Three things were owed and named in advance. **ADR 0817 §**: `pdf_syntax::serialize` "generates
no object stream at all", a stated cost, with the three decisions a generator owes — which
objects may share a stream, how large one may grow, what `/Extends` says — put under this verb
by name, and §7.5.7's ledger row moved to `partial` to record it. **ADR 0818 §2**: `split`
"deliberately does not prune", because a widget's field `/Parent` reaches the whole `AcroForm`
tree and thence objects belonging to pages the piece does not hold, "and the answer to it is
`optimize`'s reachability pass rather than a policy invented inside `split`". **RFC section 9**:
`optimize` is idempotent, a property gate no round could take because the verb did not exist.

## Decision

### 1. Four lossless passes, each derived from a sentence, and the verb is their union

- **Reachability.** §7.5.5's Table 15 makes `/Root` "[t]he catalog dictionary for the PDF file"
  and §7.7.2's Table 29 the root of a document's object hierarchy, so an object no path from
  that root reaches is one no reader of the output can ask for. §14.3.3's `/Info` is the
  trailer's *other* root — nothing in the catalog reaches it — so the walk starts at both. A
  trailer whose `/Root` is not "( Required; shall be an indirect reference )" is refused by
  name rather than guessed at.
- **Object streams**, §7.5.7, below.
- **Cross-reference streams**, §7.5.8, which object streams *force* rather than accompany:
  Table 18's type 2 entry is the only way to say where a compressed object is, and §7.5.4's
  twenty-byte line has no field that could. So `Options::form` is overridden where a carrier is
  generated, and §7.5.7 NOTE 3's "[u]se of compressed objects requires a PDF 1.5 PDF reader" is
  then met by the version floor §7.5.8.1 already carried.
- **Recompression**, §7.4.4.1's `FlateDecode` written rather than read, over the decoded bytes
  of whatever chain the producer used.

Two smaller ones fall out of the first, and both are §7.3.10's answer rather than a policy: a
reference to an object the document does not hold is not followed and the serializer states
`null` in its place ("[a]n indirect reference to an undefined object … shall be treated as a
reference to the null object"), and a source object whose *value* is null gets the same answer,
because the clause gives a reader no way to tell those two apart. And a stream's `/Length` is
not followed, because §7.3.8.2's number is a statement about the file being written and the
serializer re-derives it as a direct integer — the object a source stated it in would otherwise
be written and referred to by nothing.

**Measured by `tests/optimize_corpus.rs` over the whole pdf.js corpus** — the 961 documents this
verb rewrites, 122 723 732 bytes of them — with each pass switched off in turn, so the
attribution is under the same gate as the correctness assertions rather than in a script somebody
ran once:

| what the file is written with | total | saved |
|---|---|---|
| the source | 122 723 732 | — |
| the serializer copying what it was given | 136 486 450 | **−11.21%** |
| + §7.5.5 reachability pruning | 123 499 682 | −0.63% |
| + §7.4 recompression | 105 470 075 | 14.06% |
| + §7.5.7 object streams | 89 942 042 | **26.71%** |

The first row is the one worth reading twice: a serializer copy is *larger* than its source, by
more than a tenth, because a 1.5 document's compressed objects come out at the outermost level.
That is the cost ADR 0817 wrote down, measured — and it is most of what pruning is spending
itself undoing, which is why the second row is still negative.

### 2. §7.5.7's three decisions, answered in the clause's own words

**Which objects.** `serialize::packable` walks "[t]he following objects shall not be stored in
an object stream:" bullet by bullet, and *names the ones satisfied by construction* rather than
omitting them, because a rule met by accident is a rule waiting to be broken. "Stream objects"
is the one condition that has to be tested — and it is also why a carrier can always be reached
by a scan, since carriers are streams and so are never inside one another. Generation numbers
are 0 for every object this writer emits, which is also what "[t]he generation number of an
object stream and of any compressed object shall be zero" requires. There is no encryption
dictionary because the serializer emits no `/Encrypt`, which is also how Errata Collection 3's
Issue #439 bullet — an *encrypted* document's catalog — is met; in an unencrypted file the
catalog may be compressed, and it is. "An object representing the value of the Length entry in
an object stream dictionary" cannot exist because `/Length` is written direct. The linearized
bullet is conditional on a construct `CLAUDE.md` excludes until Annex F is separately ratified,
so it does not bind, and it is named so that whoever ratifies Annex F finds it. Finally, the
sentence further down the clause — "[a]n object in an object stream shall not consist solely of
an object reference" — is a rule about the value, so an `Object::Reference` stays outside.

**How large.** NOTE 4 states the obligation and no figure: "[t]o avoid a degradation of
performance, such as would occur when downloading and decompressing a large object stream to
access a single compressed object, the number of objects in an individual object stream needs
to be limited." Both halves of "large" are taken — a count and a byte size — and the pair is a
*measured* choice rather than a borrowed one, over every fifth document of the corpus (22.9 MB;
the ceiling is a constant rather than a flag, so each row is its own build):

| ceiling | saved over the sample |
|---|---|
| 50 objects, 16 KiB | 13.19% |
| **200 objects, 64 KiB** | **13.30%** |
| 500 objects, 256 KiB | 13.32% |

Two hundredths of a point separates the chosen pair from four times the ceiling, so the curve is
flat where it sits and the smaller pair is the one that honours NOTE 4's own reason for asking.

**What `/Extends` says.** NOTE 4's next sentence describes this writer's situation exactly —
"[t]his can require a group of object streams to be linked as a collection, which can be done by
means of the Extends entry" — so each carrier states the one before it. A chain is trivially the
"directed acyclic graph" Table 16 requires, and the entry is written rather than omitted because
the collection it describes is a real one: one document's objects, cut only by a size limit.

The header is "N pairs of integers separated by white-space" with "[t]he byte offsets … in
increasing order" by construction, and `/First` is the header's whole length, because "[a] PDF
writer shall store the first object immediately after the last byte offset separated by
white-space". NOTE 7's 2020 correction is why nothing but the next offset separates two members.

### 3. Recompression refuses wherever anything is uncertain, and that is the design

The exclusion permits recompression "without reinterpretation", so a re-encoding is legitimate
exactly when the decoded bytes are provably the same bytes. Seven conditions carry instead:
a stream whose data lives in another file (§7.3.8.2's `/F`), one the document could not decrypt,
one stating `/Length 0` with no bytes (§7.3.8.2 makes that a producer's deliberate silence), a
filter this tree does not decode or whose decode refused, a decode that came back **damaged** —
re-encoding those would write a whole stream over a truncated one and lose the fact — a `/Crypt`
filter anywhere in the chain, and a stream whose `/Filter` or `/DecodeParms` is *indirect*.

The last is about the closure rather than the bytes, and it is the one that made idempotence
true: a re-encoded stream states those entries directly, so the objects the source stated them
in would be written and referred to by nothing, and a second `optimize` would prune them.

An **image codec stops the walk instead of refusing it**, and the tail from there on is kept:
`[/ASCII85Decode /DCTDecode]` becomes `[/FlateDecode /DCTDecode]` with the producer's JPEG bytes
untouched inside. `Document::image_stream`'s reading of the same chains — "[o]nly the last entry
can be a codec" — is what says where to stop.

**A stream that fails to shrink keeps what its producer wrote.** qpdf's rule for
`--optimize-images`, and the right rule for every stream: a verb called `optimize` that made a
file larger would be a defect, and one that made a *stream* larger while the file shrank would
be a defect nobody could see. It is also half of why the verb is idempotent — a second pass over
this writer's own output finds nothing left to save and counts nothing.

**The cost is stated because it breaks the serializer's one memory property.** ADR 0817: "a
stream's data crosses from the source's `Arc<[u8]>` straight to the sink, still encoded — never
decoded, never examined." Under `Streams::Recompress` a stream is decoded and deflated in
memory, so the peak is two copies of the largest stream the output holds, bounded by the source
document's own `Limits::max_stream_len`. The decode is run against `filter::decode_with_parms`
directly rather than through `Document::decoded_stream_data`, because that route memoises and a
caller recompressing every stream would leave the whole decoded document in the memo.

### 4. zlib level 9, measured, and `zopfli` is a dependency nobody has argued

RFC section 6.5 left the effort open — "default compression effort (zlib 9 vs `zopfli`-class —
measure first, principle 2's rule)". Over the same sample, level 9 saves 13.30% against level
6's 12.60%: seven tenths of a point, on files whose reason for existing is to be smaller.
`optimize` is not on a latency path — nothing waits for it the way a first page does — so the
bytes win, `--compression-level` is there for a caller who disagrees, and `zopfli` stays a
`doc/stack.md` question nobody has asked.

### 5. Lossy image optimisation is **not** taken, and is named rather than omitted

RFC section 6.5 proposes `--images downsample=…,quality=…` and section 13's second question
makes it conditional on a **DCT encoder this tree does not have** — `zune-jpeg` decodes only.
The decision is not "later, when we get to it"; it is that without an encoder the feature cannot
be honest. "Recompress as DCT where smaller" cannot be done at all, and downsampling to
`FlateDecode`-compressed raw samples makes a photograph *larger*, so qpdf's keep-the-original
rule — which this verb adopts for every stream — would keep every image, and the flag would be a
switch that does nothing while claiming to do something. So there is no flag: `--images`
anything is a usage refusal naming the dependency and the RFC question, which is the same
discipline `--password`'s absence gets. `doc/todo/57` carries it with the dependency it waits on.

**And linearisation is refused by name.** `CLAUDE.md`: "Annex F stays excluded until
linearisation is separately ratified." `--linearize` prints that sentence and exits 1.

### 6. The verb is Table 22's `Operation::Assemble`, and the policy is asked

There is no Table 22 bit for "make this smaller". Bit 11 is "[a]ssemble the document (insert,
rotate, or delete pages …)", and a rewritten file is the document's own pages assembled into a
new one — the same pages, in the same order, stated more compactly. Answering `None` would have
made the one verb whose whole output is a derived file the one verb no policy is asked about.

An encrypted source produces an unencrypted output, because the serializer emits no `/Encrypt`;
`Assembly::has_encrypted_source` is the question, and the answer is a warning rather than a
silence.

### 7. Four things the corpus walk found, and one of them was silent corruption

None was found by reading. Each is recorded because each is a *shape* rather than an incident.

- **A dangling reference's null was written out, and a second pass could see that it had been.**
  §7.3.10's answer to a reference the assembly does not hold is `null`, and ADR 0817's serializer
  wrote `/Absent null` into the dictionary. But §7.3.7 says "[a] dictionary entry whose value is
  null (see 7.3.9, "Null object") shall be treated the same as if the entry does not exist", and
  `pdf_syntax::parser` already drops a direct null on the way *in* for that exact sentence. So the
  writer and the reader of one program disagreed about what the file said, invisibly by the clause
  and visibly in bytes on a second pass. The entry is now dropped; `Written::dangling` still counts
  it. **This supersedes a session-886 assertion on its own terms** — `tests/serialize.rs`'s
  dangling test asserted the bytes contained `/Absent null `, and its own comment already quoted
  §7.3.7.
- **A recompressed image's `/DecodeParms` was rebuilt in the *source's* numbering.** The tail of a
  chain stopped at an image codec was reconstructed from `Document::decode_parms`, whose values
  are the source's — so `bitmap-p32-eof.pdf`'s `/DecodeParms << /JBIG2Globals 3 0 R >>` came out
  naming whatever object 3 had become, and the globals the image needed were written and referred
  to by nothing. **No raster comparison saw it**, which is the whole argument of ADR 0843. The
  tail now comes out of the renumbered dictionary.
- **Discarding a parameter can strand the object it was stated in.** `issue5280.pdf` states
  `/DecodeParms` as an array whose one element is an indirect `<< /Colors 3 /Columns 60 /Predictor
  15 >>`; recompressing consumed that stage and dropped the parameter, leaving the dictionary
  object unreferenced. The rule is the same one `/Length` already gets, one entry over: nothing
  this writer stops referring to may still be in the file. A stream whose *discarded* parameters
  name anything is carried instead.
- **Four documents rewrote into files with no page at all**, and every one was a file this tree
  opens only by *recovering* what its own trailer misstates: `/Root` naming an object that is not
  there, `/Root` naming a §14.3.3 information dictionary because every object is misfiled, or a
  catalog whose `/Pages` names nothing. `pdf_model::Pages` finds their pages by looking for what
  Table 31 describes, which is right for a reader and wrong for a writer: a rewrite of a
  reconstruction is a file stating a structure no producer wrote. Refused by name, citing Table 15
  and Table 29. Six of the corpus's 974 documents are refused, and each says which clause.

## Consequences

- `crates/pdf-syntax/src/serialize.rs` gains `Options`, `ObjectStreams`, `Streams`, the
  §7.5.7 writer and the recompressor; `serialize` takes `Options` in place of a bare `Form`, and
  `split`, `merge` and `pages` pass `Options::new(form)`, which is the pass-through default.
  `Written` gains `object_streams`, `compressed`, `recompressed` and `saved`.
- `Document::filter_chain`, `Document::decode_parms` and `Document::states_no_data` widen from
  private to `pub(crate)`, so that the recompressor reads a chain the way every other caller in
  the crate does rather than by a second implementation of Table 5's rules.
- `crates/pdf-transform/src/optimize.rs`, `Plan::Optimize`, `Origin::Optimized` with a `Savings`
  the report prints per category, and the CLI's `optimize` verb with `--no-prune`,
  `--object-streams`, `--recompress`, `--compression-level` and the two usage refusals.
- **A dictionary entry whose renumbered value is null is no longer written**, which changes
  `split`, `merge` and `pages` too: their outputs stop stating the entries a §7.3.10 null makes
  absent. Nothing a reader sees changes, by §7.3.7's own sentence.
- `Output::to_json`'s origin arm moved onto `Origin` itself, and `Savings` states its own
  fields, because one function stating eight origins had outgrown the line limit.
- §7.5.7's ledger row is `implemented` and no longer records a debt; §7.5.8, §7.5.8.3, §7.4.4.1
  and §7.5.5 each gain the writer's half.
- **Three claims about this tree had decayed and are corrected**, all from session 897's carry:
  `pages.rs`'s "§14.7's structure tree, said plainly — **No verb of this suite carries it**",
  `split.rs`'s not-carried list, `doc/todo/02`'s change-map row calling the tree "absent", and
  three paragraphs of the CLI's `--help`. The tree has been carried since ADR 0834, and four
  places went on saying otherwise for three rounds.

## Alternatives not taken

- **A pruning module shared by every verb.** ADR 0818 declines to prune inside `split` on
  jurisdictional grounds, and the natural reading is that the policy should live somewhere all
  four verbs can reach. It does not, and the reason is that the walks are different walks:
  `split`'s stops at a page or a page-tree node so that `/Parent` does not drag the whole
  document in, and `optimize`'s must not stop anywhere at all. What ADR 0818 asked for was a
  *place to send the over-copy*, and running `optimize` on a piece is that place.
- **Pruning as a fixed behaviour with no switch.** Rejected: it is the one pass here that can
  change what a document *holds* rather than how it states it, and it rests on this program's
  reading of what makes an object reachable. `--no-prune` is where a caller who does not want to
  rest on that stands, and it is also what let the corpus walk attribute the savings.
- **Compressing the cross-reference stream's own data.** It is written uncompressed, as before.
  The objects it points into are compressed already, and a compressor over the table itself buys
  a fraction of a percent of a file whose object streams *are* the saving.
- **Dropping `/Extends`.** It is optional, this tree's reader ignores it, and omitting it would
  have saved a few bytes per carrier. Written anyway, because NOTE 4 describes this exact
  situation and a collection that says it is one is a collection a §7.5.7 reader can treat as one.
