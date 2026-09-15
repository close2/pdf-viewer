//! ISO 32000-2 §12.8's digital signatures: the dictionaries, and the cryptography that answers
//! them.
//!
//! One clause, and everything it needs. §12.8.1 states the work in a sentence — "[t]he signer's
//! certificate shall be determined and verified … The digest shall be recomputed and compared
//! with the one stored in the document" — and that sentence is this crate's boundary: Table 255's
//! dictionary and §12.8.1's `/ByteRange` on one side, and on the other the encodings and the
//! arithmetic that a recomputed digest has to be checked against. Nothing else is here.
//!
//! # What is here, in the order a signature is read
//!
//! [`signature`] is the clause: Table 255's dictionary, §12.8.2.2's `/DocMDP` level, §12.8.6's
//! usage rights, §12.8.4's document security store, and the three questions §12.8.1 divides the
//! subject into — has the document changed, does the signature verify, is the signer trusted —
//! of which this program answers the first two and reports the third as unanswered. Its module
//! documentation says what each answer proves, which is less than the words usually suggest.
//!
//! [`cms`] reads RFC 5652's `SignedData` out of §12.8.3.3's signature value, [`x509`] reads RFC
//! 5280's certificate out of that, [`trust`] builds and validates a certification path from them
//! (RFC 5280 section 6.1), [`revocation`] reads §12.8.4's CRLs and OCSP responses and applies them
//! to that path, and [`der`] is the X.690 tag-length-value reader all of them are built on — the
//! tree's only ASN.1. Then one module per family Table 260 names: [`pkcs1`] and
//! [`pss`] for RFC 8017's two RSA paddings, [`dsa`], [`ecdsa`], and [`eddsa`] for the row ISO/TS
//! 32002 section 5.1.2 adds. `bigint` is the seam over `crypto-bigint` that keeps the budgets
//! and the refusal names this project's own while the multiplications are reviewed code.
//!
//! # Why this is not part of the document model
//!
//! `pdf-model`'s stated responsibility is the page tree, and none of the nine modules below
//! [`signature`] contains a line of PDF. They are ASN.1, X.509 and modular arithmetic, which is
//! the argument `doc/PLAN.md` already records for a crate boundary — "self-contained,
//! independently testable and independently fuzzable" — and the same argument that made
//! `pdf-syntax` separate in the first place. `doc/reviews/984-direction-and-boundaries.md`
//! finding 1 is where it was applied to this stack; ADR 1020 is the extraction, and states what
//! it did and did not buy.
//!
//! The dependency runs *from* `pdf-model` *to* here and not the other way, which is what keeps
//! the boundary checkable: a signature is read out of the object graph `pdf-syntax` gives, never
//! out of a page. §12.8.2.2's `/DocMDP` level and §12.8.6's usage rights are what `pdf-model`
//! comes here for, because what a document says may be changed without invalidating its author's
//! signature is a restriction on the reader, and `pdf_model::restriction` is where those are
//! collected.
//!
//! # Nothing here is called valid
//!
//! Deliberate, and stated in [`signature`] at length: this program has no trust store and no
//! network, so §12.8.1's third question is not answered, and a word that implies it was answered
//! is not used. ADR 0215 separated the three questions; ADRs 0229, 0314, 0322 and 0532 answered
//! the second for each family in turn.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod bigint;
pub mod cms;
pub mod der;
pub mod dsa;
pub mod ecdsa;
pub mod eddsa;
pub mod ess;
pub mod pkcs1;
pub mod pss;
pub mod revision;
pub mod revocation;
pub mod signature;
pub mod timestamp;
pub mod trust;
pub mod verdict;
pub mod x509;
