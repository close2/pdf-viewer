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
//! subject into — has the document changed, does the signature verify, is the signer trusted.
//! Its module documentation says what each answer proves, which is less than the words usually
//! suggest; the third is [`trust`]'s and [`verdict`]'s, under anchors a host supplies.
//!
//! [`cms`] reads RFC 5652's `SignedData` out of §12.8.3.3's signature value, [`x509`] reads RFC
//! 5280's certificate out of that, [`ess`] reads RFC 5035's signing-certificate attribute that
//! §12.8.3.4.5 step a) compares against it, [`trust`] builds and validates a certification path
//! from them (RFC 5280 section 6.1), [`revocation`] reads §12.8.4's CRLs and OCSP responses and
//! applies them to that path, and [`der`] is the X.690 tag-length-value reader all of them are
//! built on — the tree's only ASN.1. Then one module per family Table 260 names: [`pkcs1`] and
//! [`pss`] for RFC 8017's two RSA paddings, [`dsa`], [`ecdsa`], and [`eddsa`] for the row ISO/TS
//! 32002 section 5.1.2 adds. `bigint` is the seam over `crypto-bigint` that keeps the budgets
//! and the refusal names this project's own while the multiplications are reviewed code.
//!
//! Three modules answer questions the clause asks after the arithmetic. [`revision`] is
//! §12.8.2.2.2's second step — the signed revision beside the current one, object by object,
//! ranked against Table 257's `/P`. [`timestamp`] establishes §12.8.5's document timestamps, so
//! that a verdict has an instant to be asserted as of. [`policy`] reads §12.8.3.4.4's signature
//! policy identifier and whether the document the file carries is the one the signer committed
//! to, as far as the texts this tree holds reach.
//!
//! # Why this is not part of the document model
//!
//! `pdf-model`'s stated responsibility is the page tree, and only [`signature`], [`revision`]
//! and [`timestamp`] here read a file at all: everything they are built on is ASN.1, X.509 and
//! modular arithmetic over bytes somebody else handed it. That is
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
//! # One type may say *valid*, and it cannot be made without the proof
//!
//! §12.8.1's three questions all have an answer here, and the word that joins them is a type
//! rather than a convention every module has to keep. [`verdict::Valid`] has no public
//! constructor and no public field, [`verdict::Verdict::reached`] is the only function in this
//! tree that makes one, and it takes a [`verdict::Anchored`] — which only a [`trust::Trust`] that
//! reached an anchor can produce. So a caller holding no anchor cannot reach the word by any
//! path, including a wrong one.
//!
//! The anchors are an input and never this crate's: RFC 5280 section 6.1.1 makes them input (d)
//! and says the choice is the verifier's policy, so [`trust`] holds no certificate list, opens no
//! socket and asks no clock. With none supplied the answer is [`trust::Trust::NoAnchorSupplied`],
//! which is a statement about this program rather than about the signature. Every verdict is
//! asserted as of an instant and carries which one, because *valid* with no instant reads as
//! *valid now*, and that is the one thing this crate never establishes.
//!
//! [`verdict`] states what the word is claimed to mean and what it is not. ADR 0215 separated the
//! three questions; ADRs 0229, 0314, 0322 and 0532 answered the second for each family in turn;
//! ADR 1039 made the anchor a host's to supply, ADR 1067 the revocation material the document's
//! own, and ADR 1076 the word a type.

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
pub mod policy;
pub mod pss;
pub mod revision;
pub mod revocation;
pub mod signature;
pub mod timestamp;
pub mod trust;
pub mod verdict;
pub mod x509;
