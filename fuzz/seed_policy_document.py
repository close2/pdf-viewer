#!/usr/bin/env python3
"""Seed the `policy_document` fuzz target with one stored signature policy document per shape.

    python3 fuzz/seed_policy_document.py fuzz/corpus/policy_document

The target reads its bytes as §12.8.3.4.4's stored policy document under each of the three
syntaxes, so the seeds are documents of all three, written for this file: ETSI TS 119 172-2
clause 4.2's XML `SignaturePolicy` holding one clause 4.14 `PolicyRule` of each `BasicRule` kind
(the shapes `policy_rules/ts_119_172_2/tests.rs` judges), once more with every rule at once and
once in a prefixed namespace without its final `#`; ETSI TS 119 172-3 clause 4.2's ASN.1
`SignaturePolicy` with its `Digest` computed over its components as clause 4.3.2 says; and RFC
3125 section 5's `SignaturePolicy`, the syntax ETSI TR 102 272 clause 6 restates, its
`signPolicyHash` computed over `signPolicyInfo`. No corpus document stores one, which is why
these are composed rather than taken from a file.
"""

import hashlib
import os
import sys

NAMESPACE = "http://uri.etsi.org/19172/v1.1.1#"
SHA256_URI = "http://www.w3.org/2001/04/xmlenc#sha256"
RSA_SHA256 = "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"

RULES = [
    "<DataToBeSignedRules><AnyOfMimeType>application/pdf text/xml</AnyOfMimeType></DataToBeSignedRules>",
    "<SigToDTBSRelationRules><DTBSCardinality><MinimumValue qualifier=\"HigherThan\">1</MinimumValue>"
    "</DTBSCardinality><SigDTBSRelativePosition>EnvelopedSig</SigDTBSRelativePosition></SigToDTBSRelationRules>",
    "<SigFormatsAndLevels><SigFormats><Format>http://uri.etsi.org/ades/format/pades</Format></SigFormats>"
    "</SigFormatsAndLevels>",
    "<AugmentationRules><PreviousValidationRequired>true</PreviousValidationRequired>"
    "<LevelId>urn:level</LevelId></AugmentationRules>",
    "<SigningCertRules><MandatedSigningCertInfo>signingCertOnly</MandatedSigningCertInfo>"
    "<SigningCertTrustConditions/></SigningCertRules>",
    "<TimeEvidencesRules><RulesForSetOfEvidences><EvidenceIdentifiers><Identifier>urn:tst</Identifier>"
    "</EvidenceIdentifiers><LevelOfAssurance>urn:high</LevelOfAssurance><TimeStampTrustCondition>"
    "<SignatureTimeStampDelay><DeltaSeconds>60</DeltaSeconds><DeltaMinutes>0</DeltaMinutes>"
    "<DeltaHours>0</DeltaHours><DeltaDays>0</DeltaDays></SignatureTimeStampDelay>"
    "</TimeStampTrustCondition></RulesForSetOfEvidences></TimeEvidencesRules>",
    "<SignerAttributesConstraints><NoSignerAttributesAllowed/></SignerAttributesConstraints>",
    "<QualifyingPropertiesRules><LevelRules><LevelIdentifier>urn:b-b</LevelIdentifier></LevelRules>"
    "</QualifyingPropertiesRules>",
    "<SCDLoARules>urn:qscd</SCDLoARules>",
    "<CryptoSuitesRules><AlgConstraints><AlgId>" + RSA_SHA256 + "</AlgId><Usages>"
    "http://uri.etsi.org/19172/v1.1.1/Usage#SigningCert</Usages><ExpirationDate>2030-01-01T00:00:00Z"
    "</ExpirationDate><MinKeyLength>2048</MinKeyLength></AlgConstraints><AlgConstraints><AlgId>"
    + SHA256_URI + "</AlgId></AlgConstraints></CryptoSuitesRules>",
    "<CommitmentRules><ComitmentRulesComponent MatchingIndicator=\"all\"><Commitment><Identifier>"
    "urn:oid:1.2.3.5</Identifier></Commitment><DataToBeSignedRules><AnyOfMimeType>text/xml"
    "</AnyOfMimeType></DataToBeSignedRules></ComitmentRulesComponent></CommitmentRules>",
    "<OtherRule><x:Rule xmlns:x=\"urn:extension\"/></OtherRule>",
]


def xml(rules, namespace=NAMESPACE, prefix="", scope=None):
    p = f"{prefix}:" if prefix else ""
    declared = f'xmlns{":" + prefix if prefix else ""}="{namespace}"'
    attribute = f' RecommendedScope="{scope}"' if scope else ""
    body = "".join(f"<PolicyRule{attribute}>{rule}</PolicyRule>" for rule in rules)
    if prefix:
        # The rules' own elements are in the policy namespace too, so they take the prefix.
        body = body.replace("</", "\0").replace("<", f"<{p}").replace("\0", f"</{p}")
        body = body.replace(f"{p}x:", "x:")
    return (
        f'<?xml version="1.0" encoding="UTF-8"?>\n<{p}SignaturePolicy {declared} '
        'xmlns:ds="http://www.w3.org/2000/09/xmldsig#">'
        f'<{p}Digest><ds:DigestMethod Algorithm="{SHA256_URI}"/>'
        "<ds:DigestValue>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</ds:DigestValue>"
        '<ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>'
        f"</{p}Digest><{p}PolicyComponents><{p}GeneralDetails><{p}SigPolicyDetails>"
        f"<{p}PolicyIdentifier>urn:oid:1.2.3.4</{p}PolicyIdentifier></{p}SigPolicyDetails>"
        f"<{p}OtherDetails><{p}DateOfIssue>2025-12-01T00:00:00Z</{p}DateOfIssue><{p}SigningPeriod>"
        f"<{p}NotBefore>2026-01-01T00:00:00Z</{p}NotBefore><{p}NotAfter>2026-06-01T00:00:00Z"
        f"</{p}NotAfter></{p}SigningPeriod></{p}OtherDetails></{p}GeneralDetails>"
        f"<{p}PolicyRules>{body}</{p}PolicyRules></{p}PolicyComponents></{p}SignaturePolicy>"
    ).encode("utf-8")


def tlv(tag, contents):
    length = len(contents)
    if length < 0x80:
        head = bytes([length])
    else:
        octets = length.to_bytes((length.bit_length() + 7) // 8, "big")
        head = bytes([0x80 | len(octets)]) + octets
    return bytes([tag]) + head + contents


def seq(*parts):
    return tlv(0x30, b"".join(parts))


SHA256_OID = tlv(0x06, bytes.fromhex("608648016503040201"))
POLICY_OID = tlv(0x06, bytes.fromhex("2a0304"))


def time(spelled):
    return tlv(0x18, spelled.encode("ascii"))


def ts_119_172_3():
    """Clause 4.2's `SignaturePolicy`: a `Digest` over `PolicyComponents`, one basic rule."""
    period = seq(time("20260101000000Z"), time("20260601000000Z"))
    details = seq(time("20251201000000Z"), tlv(0xA0, period))
    general = seq(
        seq(POLICY_OID, seq(seq(tlv(0x13, b"en"), tlv(0x0C, b"seeds")))),
        tlv(0xA1, details),
    )
    rule = seq(tlv(0xA1, tlv(0xA4, tlv(0x0A, b"\x01"))))
    components = seq(general, seq(rule))
    digest = hashlib.sha256(components).digest()
    return seq(seq(seq(SHA256_OID), tlv(0x04, digest)), components)


def tr_102_272():
    """RFC 3125 section 5's `SignaturePolicy`, its `signPolicyHash` over `signPolicyInfo`."""
    issuer = seq(tlv(0x86, b"urn:seeds"))
    common = seq(tlv(0xA0, seq(tlv(0xA0, seq(seq(tlv(0x06, bytes.fromhex("2a864886f70d010903"))))))))
    commitment = seq(seq(seq(tlv(0x05, b""))))
    validation = seq(seq(time("20260101000000Z"), time("20260601000000Z")), common, commitment)
    info = seq(POLICY_OID, time("20251201000000Z"), issuer, tlv(0x0C, b"seeds"), validation)
    return seq(seq(SHA256_OID), info, tlv(0x04, hashlib.sha256(info).digest()))


def documents():
    for rule in RULES:
        yield xml([rule])
        yield xml([rule], scope="generation")
    yield xml(RULES)
    yield xml(RULES, namespace=NAMESPACE.rstrip("#"), prefix="sp")
    yield ts_119_172_3()
    yield tr_102_272()


def main(argv):
    if len(argv) != 2:
        sys.exit(__doc__)
    directory = argv[1]
    os.makedirs(directory, exist_ok=True)
    written = 0
    for data in documents():
        name = os.path.join(directory, hashlib.sha256(data).hexdigest())
        if not os.path.exists(name):
            with open(name, "wb") as handle:
                handle.write(data)
            written += 1
    print(f"seed_policy_document.py: {written} new seeds in {directory}")


if __name__ == "__main__":
    main(sys.argv)
