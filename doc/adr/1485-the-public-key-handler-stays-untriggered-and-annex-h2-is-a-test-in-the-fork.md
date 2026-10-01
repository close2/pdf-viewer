# 1485 — The public-key handler stays untriggered, and Annex H.2 is a test in the codec's fork

Session 1325. Status: accepted. Code: `doc/patches/hayro-jbig2-annex-h2-test.patch` (new). Rows:
§7.6.5, §7.6.5.1, §7.6.5.2, §7.6.5.3 stay `reported`; §7.6 and §7.6.6 stay `partial`.

## 1. What the owner's two answers decide

This round was briefed to build §7.6.5's reading path if the owner's answers said the trigger was
met. They say it is not, in the owner's words:

- `doc/questions/A66`: "Shared crate below both. Wait for a real trigger." Where the stack lives is
  settled (one crate below `pdf-syntax` and `pdf-signature`, holding the `der`/`cms`/`x509`/`bigint`
  seam with `EnvelopedData` and RSA decryption added there), and the build waits for a document
  whose recipient list could match a certificate a user holds, or a host asking to supply a key.
- `doc/questions/A168`: "I agree with your recommendation (including the extra thing)." The
  recommendation it ratifies: a published test key is held by nobody in particular, so the PDFBox
  documents whose keystores ship beside them are **not** the trigger; they are the end-to-end
  witness set a future build is tested against. The extra thing was the census correction.

Everything `A168`'s `Owes:` line asks for is already in the tree: the §7.6.5 ledger note names the
witness set, `A66`'s reading carries the correction, `doc/todo/65`'s fifth bucket says the build
stays untriggered, and `crates/pdf-syntax/tests/public_key_witnesses.rs` (4 tests, passing) holds
each witness's recipient to its keystore's certificate, `PDFBOX-4421-0` to `-3` included. So nothing
is built, no row moves, and no keystore was fetched: the one the brief named is already
`doc/corpora/pdfbox/.../PDFBOX-4421-keystore.pfx`. A later round should not re-open this until one
of `A66`'s two arms is lit.

## 2. Annex H.2 runs where the decoder is

T.88 Annex H.2 gives a test sequence for the arithmetic coder: 256 decisions under one context,
packed into 32 bytes, the 30 bytes they encode to, and Table H.1, the registers before each
decision. The brief asked for it as a `pdf-sandbox` fixture. **It cannot be one**: `hayro-jbig2`'s
`ArithmeticDecoder` and its context are `pub(crate)`, and no public route decodes under a single
fixed context (a generic region's context moves with every pixel it decodes). Writing a second MQ
decoder in the test would test the test. So the fixture is a unit test inside the codec, beside its
decoder, delivered as the third patch under `doc/patches/` for the owner's fork. `tools/main-checkout.py`
lists it until the pinned `rev` moves.

The test decodes the 30 bytes and, before each of the 256 decisions, compares I(CX), MPS(CX), A, CT
and C with Table H.1's row (C is the last column, the software-conventions decoder of Annex G,
which is the form `hayro-jbig2` implements), then compares the decision. The decisions are also
checked against the 32 packed bytes, so a slip in transcribing either fails. At the pinned base
revision it passes; with one Qe entry changed (0x5101 to 0x5102) it fails at event 95 on A. It
changes no code.

**Why the numbers may be in the patch.** T.88's text is cited and paraphrased, never quoted
(`doc/third-party-data.md`), and Annex K's conformance files are held outside the tree because
their own notice is narrower than this repository's licence. H.2 is different in kind: two byte
strings and the register states that T.88's decoding procedures compute from them. They are a test
vector, data that any implementation of the procedures reproduces, and not ITU's prose. The patch's
comments paraphrase; nothing in it is quoted. If the owner reads this differently, the patch is the
only place the numbers are, and deleting it removes them.

`hayro-jpeg2000` carries a copy of the same decoder (`j2c/arithmetic_decoder.rs`), identical except
for two `reset` methods on the context. The patch does not touch it; the same test would apply.
