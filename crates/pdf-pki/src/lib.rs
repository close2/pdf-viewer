//! The public-key encodings below `pdf-syntax` and `pdf-signature`: X.690's DER, RFC 5280's
//! certificate, RFC 5652's enveloped data, RFC 8017's RSA decryption and RFC 5753's elliptic-curve
//! key agreement.
//!
//! # Why this crate exists
//!
//! Two clauses of ISO 32000-2 read the same encodings from opposite ends of the crate graph.
//! §12.8's signatures read RFC 5652's `SignedData` and the certificates inside it, and are
//! `pdf-signature`, which is built over `pdf-syntax`'s object reader. §7.6.5's public-key
//! security handler reads RFC 5652's `EnvelopedData` and matches its recipients against a
//! certificate, and is part of opening a document — `pdf-syntax`'s `crypt` module, *below*
//! `pdf-signature`. Neither can call the other's reader without a cycle, and a second ASN.1 reader
//! on the crate that touches every byte of every untrusted document is the one thing
//! `doc/stack.md`'s rule forbids. So the reader is here, once, under both: one implementation, one
//! fuzz surface, one audit point (the owner's answers A66 and A800, ADR 2801).
//!
//! # What is here
//!
//! [`der`] is the X.690 tag-length-value reader everything else is built on — the tree's only
//! ASN.1. [`bigint`] is the seam over `crypto-bigint` that keeps the budgets and the refusal names
//! this project's own while the multiplications are reviewed code. [`x509`] reads RFC 5280's
//! certificate for what matching a recipient and verifying a signature need from it.
//! [`enveloped`] reads RFC 5652 section 6's `EnvelopedData` and opens it for a recipient, with
//! what opening needs beside it: [`key`], the recipient's private key of either family; [`rsa`],
//! RFC 8017's private key, RSADP, and the `RSAES-PKCS1-v1_5` decoding a key-transport recipient
//! uses; [`ec`], RFC 5915's elliptic-curve private key and SEC 1's Diffie–Hellman primitive a
//! key-agreement recipient uses; and [`key_wrap`], RFC 3394's AES key unwrap, which carries a
//! key-agreement recipient's content-encryption key.
//!
//! Everything here reads bytes somebody else handed it, under `#![forbid(unsafe_code)]`,
//! allocating nothing a length in the input sizes beyond the input itself.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bigint;
pub mod der;
pub mod ec;
pub mod enveloped;
pub mod key;
pub mod key_wrap;
pub mod rsa;
pub mod x509;
