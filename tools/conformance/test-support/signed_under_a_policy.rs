//! A one-page document signed under an explicit signature policy whose one rule takes its trust
//! anchors from a trusted list the policy references, generated for the windows' Shift + T tests.
//!
//! ISO 32000-2 §12.8.3.4.4 makes ETSI EN 319 122-1 clause 5.2.9's signature-policy-identifier apply
//! to a `PAdES-E-EPES` signature; ETSI TS 119 172-3's `TAsInTrustedList` names the list by its
//! `TLReference`, and TS 119 172-2 clause 4.22.2.1 b) determines that trust point's anchors from
//! the list referenced, which is why a bound policy's references go with a refresh of the trusted
//! lists (ADR 2936). Every byte is built here: no real trusted list and no corpus document, and the
//! reference names a host under `.invalid` (RFC 2606), which no refresh can reach.
//!
//! Included by `#[path]` from each window's tests, as `gpu_lock_hook.rs` is, so that four windows
//! drive the one document rather than four copies of it. Nothing in the signature verifies: the
//! policy is read and enforced whether or not it does (`viewer-host`'s `signature_policy.rs`).

use std::fmt::Write as _;

/// The list the policy's one trust point references.
pub(super) const REFERENCE: &str = "https://lists.invalid/tl.xml";

/// The policy document, a TS 119 172-3 XML signature policy whose `SignerTrustTrees` hold one
/// `TAsInTrustedList` naming [`REFERENCE`] for the qualified CA service type.
#[must_use]
pub(super) fn policy() -> Vec<u8> {
    format!(
        r#"<SignaturePolicy xmlns="http://uri.etsi.org/19172/v1.1.1#" xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
  <Digest>
    <ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
    <ds:DigestValue>AAAA</ds:DigestValue>
    <ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>
  </Digest>
  <PolicyComponents>
    <GeneralDetails>
      <SigPolicyDetails><PolicyIdentifier>urn:oid:2.16.724.1.3.1.1.2.1.9</PolicyIdentifier></SigPolicyDetails>
      <OtherDetails><DateOfIssue>2025-12-01T00:00:00Z</DateOfIssue>
        <SigningPeriod><NotBefore>2026-01-01T00:00:00Z</NotBefore></SigningPeriod></OtherDetails>
    </GeneralDetails>
    <PolicyRules><PolicyRule><SigningCertRules><SigningCertTrustConditions><SignerTrustTrees>
      <CertificateTrustPoint><TrustAnchors><TAsInTrustedList>
        <TLReference>{REFERENCE}</TLReference>
        <ServiceTypes><tsl:URI xmlns:tsl="http://uri.etsi.org/02231/v2#">http://uri.etsi.org/TrstSvc/Svctype/CA/QC</tsl:URI></ServiceTypes>
      </TAsInTrustedList></TrustAnchors></CertificateTrustPoint></SignerTrustTrees>
      <SignerRevTrust><CertificateRevReq><EndRevReq>nocheck</EndRevReq><CACerts>nocheck</CACerts>
      </CertificateRevReq></SignerRevTrust></SigningCertTrustConditions></SigningCertRules>
    </PolicyRule></PolicyRules>
  </PolicyComponents>
</SignaturePolicy>"#
    )
    .into_bytes()
}

/// One DER value: a tag, X.690's shortest definite length, and the contents.
fn der(tag: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let length = contents.len();
    if let Ok(short @ 0..0x80) = u8::try_from(length) {
        out.push(short);
    } else if let Ok(one) = u8::try_from(length) {
        out.extend([0x81, one]);
    } else {
        let two = u16::try_from(length).unwrap_or(u16::MAX);
        out.push(0x82);
        out.extend(two.to_be_bytes());
    }
    out.extend_from_slice(contents);
    out
}

/// A CMS `SignedData` whose one `SignerInfo` signs EN 319 122-1 clause 5.2.9.1's
/// signature-policy-identifier with the SHA-1 of `policy` and clause 5.2.9.2's URL qualifier.
fn policy_cms(policy: &[u8]) -> Vec<u8> {
    const SIGNED_DATA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02];
    const DATA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01];
    const SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
    const SHA1: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];
    const RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const SIG_POLICY_ID: &[u8] = &[
        0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x02, 0x0f,
    ];
    const SPQ_URI: &[u8] = &[
        0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x05, 0x01,
    ];
    const POLICY_OID: &[u8] = &[0x60, 0x85, 0x54, 0x01, 0x03, 0x01, 0x01, 0x02, 0x01, 0x09];
    let algorithm = |oid: &[u8]| der(0x30, &der(0x06, oid));
    let digest = pdf_signature::cms::Digest::Sha1.compute(&[policy]);
    let qualifier = der(
        0x30,
        &[
            der(0x06, SPQ_URI),
            der(0x16, b"http://127.0.0.1:9/policy.xml"),
        ]
        .concat(),
    );
    let identifier = [
        der(0x06, POLICY_OID),
        der(0x30, &[algorithm(SHA1), der(0x04, &digest)].concat()),
        der(0x30, &qualifier),
    ]
    .concat();
    let attribute = der(
        0x30,
        &[der(0x06, SIG_POLICY_ID), der(0x31, &der(0x30, &identifier))].concat(),
    );
    let signer_info = der(
        0x30,
        &[
            der(0x02, &[1]),
            der(0x30, &[der(0x30, &[]), der(0x02, &[1])].concat()),
            algorithm(SHA256),
            der(0xa0, &attribute),
            der(0x30, &[der(0x06, RSA), der(0x05, &[])].concat()),
            der(0x04, &[0; 8]),
        ]
        .concat(),
    );
    let signed_data = der(
        0x30,
        &[
            der(0x02, &[1]),
            der(0x31, &algorithm(SHA256)),
            algorithm(DATA),
            der(0x31, &signer_info),
        ]
        .concat(),
    );
    der(
        0x30,
        &[der(0x06, SIGNED_DATA), der(0xa0, &signed_data)].concat(),
    )
}

/// The document: a 200-unit page signed under [`policy`], its byte range ending at the revision's
/// end so §12.8.2.2.2's comparison opens it. Object 1 is the catalog, 2 the page tree, 3 the page,
/// 4 the signature and 5 its field.
#[must_use]
pub(super) fn document() -> Vec<u8> {
    let mut contents = String::new();
    for octet in policy_cms(&policy()) {
        let _ = write!(contents, "{octet:02x}");
    }
    let signature = format!(
        "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached \
         /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <{contents}> >>"
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
        signature.as_str(),
        "<< /FT /Sig /T (Signature1) /V 4 0 R /Subtype /Widget >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    // The two placeholders are this function's own text, so each is found; an absent one would
    // leave the range unfilled and the comparison would say so rather than this panic.
    let start = out
        .find("/Contents <")
        .map_or(0, |at| at.saturating_add("/Contents ".len()));
    let end = out[start..]
        .find('>')
        .map_or(start, |at| start.saturating_add(at).saturating_add(1));
    let filled = format!(
        "{:010} {:010} {:010} {:010}",
        0,
        start,
        end,
        out.len().saturating_sub(end)
    );
    if let Some(range) = out.find("/ByteRange [") {
        let at = range.saturating_add("/ByteRange [".len());
        out.replace_range(at..at.saturating_add(filled.len()), &filled);
    }
    out.into_bytes()
}

/// Binds [`policy`] to `document`'s signature in `viewer`, as a window's fetch of the policy hands
/// it on (`viewer_core::Command::PolicyCopy`, ADR 2624): the report publishes the policy the
/// signature names, and the copy whose SHA-1 the signer committed to is handed back for it.
///
/// Answers how many policies were published, so a test can say which half failed.
pub(super) fn bind(viewer: &mut viewer_core::Viewer, document: viewer_core::DocumentId) -> usize {
    let published: Vec<_> = viewer
        .handle(viewer_core::Command::Report)
        .flat_map(|event| match event {
            viewer_core::Event::SignaturePoliciesPublished { policies, .. } => policies,
            _ => Vec::new(),
        })
        .collect();
    let count = published.len();
    for policy in published {
        let _ = viewer
            .handle(viewer_core::Command::PolicyCopy {
                document,
                policy,
                copy: self::policy(),
            })
            .count();
    }
    count
}
