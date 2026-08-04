# ADR 0186 — An XML parser over untrusted bytes, and which one

Status: accepted, 2026-08-04 (session 294).

## Context

§14.3.2 says what a metadata stream contains in one sentence:

> The contents of a metadata stream shall be the metadata represented in Extensible Markup
> Language (XML) and the grammar of the XML representing the metadata shall be defined according
> to the extensible metadata platform specification (ISO 16684-1).

**319 of the 974 corpus documents carry one** — the largest single population on the handover's
"what is not implemented" table, by a factor of fifteen over the next entry — and this tree read
none of it. The reason was written down honestly and had not moved for a hundred sessions: reading
XMP is an XML parser over untrusted bytes, which is a *dependency decision* of the kind ADR 0014
took for the image codecs and ADR 0031 for the ciphers, and nobody had taken it.

What the absence cost was named rather than hidden. §12.2's `/DisplayDocTitle` asks for `dc:title`
specifically; this program showed §14.3.3's `/Info /Title` instead and said so, once for each of
the 18 documents that set the flag and carry a stream. A defensible substitution — Table 349's own
NOTE calls the two the same fact in different places — but a substitution.

## Decision

**Take the dependency, and take the smallest thing that can do the job: `xmlparser` 0.13.6.**

The four questions ADR 0014 asks of every dependency, answered:

**Which crate.** `xmlparser`: 2148 lines, `#![forbid(unsafe_code)]`, **no dependencies at all**,
MIT/Apache-2.0. It is a pull *tokenizer* — an `Iterator<Item = Result<Token>>` over a state
machine — which is exactly the shape `doc/todo/50` said to want and the opposite of the DOM
builder it said not to.

The alternative considered seriously was `quick-xml`, which is already in this tree's graph
through `wayland-scanner` and also forbids unsafe. It loses on one point and it is the point that
matters here: it depends on `memchr`, whose SIMD paths are the tree's `unsafe`, and
`wayland-scanner` is a **proc macro** — it runs at build time and reaches no shipped binary, so
taking `quick-xml` at runtime would put `memchr` into a viewer for the first time. Trading two
crates for one is not the argument; trading a build-time dependency for a runtime one over
untrusted input is.

**What its attack surface is.** The two denial-of-service shapes XML is famous for are both
structurally absent rather than defended against:

- **The billion-laughs expansion has nothing to expand.** `xmlparser` resolves no general entity.
  A `<!ENTITY lol1 "&lol;&lol;…">` declaration arrives as a `Token::EntityDeclaration` and is
  dropped; a `&lol9;` in content arrives as the eight bytes a producer wrote. `pdf_model::xmp`
  expands the five entities XML itself predefines and numeric character references, and nothing
  else — so an expansion cannot grow the input.
- **The external entity is inert.** Nothing in the crate opens a file or a socket, so a
  `SYSTEM "file:///etc/passwd"` is a string in a DTD nobody reads. Principle 3's renderer has no
  filesystem anyway; this makes it true one layer earlier.
- **Nesting costs no stack**, because the tokenizer is iterative. What nesting costs is
  `pdf_model::xmp`'s own `Vec` of frames, which is bounded at 64.

Both are tested in `xmp.rs`, by name, with the two classic payloads.

**What it does on malformed input.** Returns `Err`. It is a tokenizer, so it checks a tag's
*syntax* and never builds a tree — which means it does **not** notice that `<a></b>` is unbalanced.
That is this tree's job and `XmpError::Unbalanced` is where it is done; the variant is separate
from `Malformed` precisely so that the two readers' findings are distinguishable.

**What the fuzz target looks like.** `fuzz/fuzz_targets/xmp.rs`, seeded with all 318 packets the
corpus decodes, checking three properties: parsing terminates without panicking, the stated
budgets hold, and a successful parse is idempotent — the last because the reader carries two
stacks across a whole packet and state leaking between elements is the defect one pass cannot see.
**It found something on its first run and the finding was the target's**, not the reader's: an
assertion that a resolved namespace contains no space, refuted in seconds by a mutation that put
white space inside an `xmlns` value. XML says nothing about what a namespace name looks like, and
a reader that decided otherwise would refuse files over a rule nobody wrote. Clean at 50 000 runs
and at 1 000 000 after the assertion was corrected.

## What the reader reads, and what it says it does not

Both spellings of ISO 16684-1 section 7.5's simple property — an attribute on an `rdf:Description` and a
child element of one — and all three of its containers, `rdf:Alt`, `rdf:Seq`, `rdf:Bag`.

**A name is a namespace URI and a local part, never a prefix.** `dc:title` is not a name;
`{http://purl.org/dc/elements/1.1/}title` is. Every element and attribute is resolved through the
`xmlns` bindings in scope before it is compared with anything, and `Xmp::text` takes a URI. The
test that holds this is the packet with every prefix renamed, which must read identically.

**A structured value is recorded as present and uninterpreted** (`Value::Structure`), which is the
difference between a gap and a silence. Nothing in clause 12 or 14 asks for one.

**Two things are deliberately not converted.** An XMP date is ISO 8601 and §7.9.4's is not, so
`xmp:CreateDate` comes back as the string the file wrote rather than as a `pdf_syntax::Date`: they
are two grammars, and reshaping one into the other would answer a question about the file with a
guess. And a language alternative is matched on the exact `xml:lang` the packet states rather than
on RFC 4647's lookup, because a fallback from `en-GB` to `en` is a policy a *host* has.

## What it is worth, measured

`cargo test --release -p pdf-model --test xmp -- --ignored`, over all 974 documents:

```text
refused: the metadata stream would not decode — ["PDFBOX-3148-2-fuzzed.pdf"]
319 documents carry §14.3.2's stream: 318 read, 1 refused, 3191 properties between them
  (most in one packet: 25), 106 state dc:title
  93 state both dc:title and §14.3.3's /Title: 1 disagree
    devicen.pdf: XMP "devicen.pdf" vs /Info "devicen1.ps"
```

The one refusal is a stream that does not decode at all — a `pdf-syntax` refusal one layer below
this reader, on a file whose name says what was done to it.

**And the census answers a question this project has been carrying as a caveat.** 93 documents
state a title in both places and **one disagrees**: `devicen.pdf`, where XMP says `devicen.pdf`
and the dictionary says `devicen1.ps` — a producer that put its input file's name in one and its
output's in the other. So the substitution §12.2 has been getting for a hundred sessions was right
92 times out of 93, which is the sort of thing that is worth knowing *after* the reader exists and
could not have been known before it.

## The lesson

**A refusal whose reason is "a decision nobody has taken" is not the same as a refusal whose
reason is a clause, and only the first kind expires by being read.** This one sat for a hundred
sessions with 319 witnesses behind it, in a file that said exactly what would settle it — which
crate, what its attack surface is, what it does on malformed input, what the fuzz target looks
like. Answering those four questions took one round. The todo file was not wrong to hold it; what
is worth noticing is that **the item was blocked on nobody having asked**, and that no sweep in
`doc/todo/01` looks for that shape, because the row said `partial` and its note was true.
