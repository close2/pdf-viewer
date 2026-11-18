#!/usr/bin/env bash
# Generates the fixture of ADR 3056: a time-stamp token whose unit's certificate is issued by an
# intermediate authority under a test root, and two OCSP responses about that token's own path.
#
#   root          RSA 3072, self-signed,                2026-01-01 to 2046-01-01
#   intermediate  RSA 2048, CA, issued by root,         2026-01-01 to 2030-01-01
#   unit          RSA 3072, critical id-kp-timeStamping, issued by intermediate,
#                                                       2026-01-01 to 2031-01-01
#   unit-responder         RSA 3072, id-kp-OCSPSigning, issued by intermediate,
#                                                       2026-01-01 to 2028-01-01
#   intermediate-responder RSA 2048, id-kp-OCSPSigning, issued by root,
#                                                       2026-01-01 to 2036-01-01
#
# token.der carries the unit and the intermediate (`openssl ts -reply -chain`); the root is not
# in it, because an anchor is the reader's to supply. unit-status.der is the unit-responder's
# answer about the unit's certificate, intermediate-status.der the intermediate-responder's about
# the intermediate's: each is RFC 6960 section 4.2.2.2's criterion 3, a responder the issuer of
# the certificate in question certified. Every name says it is a test; none is a real authority.
#
# Run from this directory: `bash generate.sh`. Keys are fresh on every run, so the bytes differ,
# and the facts the tests read (sizes, validities, which certificate issued which) do not.
set -euo pipefail
cd "$(dirname "$0")"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

key() { openssl genpkey -quiet -algorithm RSA -pkeyopt "rsa_keygen_bits:$2" -out "$work/$1.key"; }
ext() { printf '%s\n' "$2" > "$work/$1.ext"; }
cert() { # name issuer not_before not_after
    openssl req -new -key "$work/$1.key" -subj "/O=quorra test fixture/CN=quorra token path $1" \
        -out "$work/$1.csr"
    openssl x509 -req -in "$work/$1.csr" -CA "$work/$2.pem" -CAkey "$work/$2.key" \
        -set_serial "0x$(openssl rand -hex 8)" -not_before "$3" -not_after "$4" -sha256 \
        -extfile "$work/$1.ext" -out "$work/$1.pem"
}

key root 3072
openssl req -new -x509 -key "$work/root.key" -subj "/O=quorra test fixture/CN=quorra token path root" \
    -set_serial 1 -not_before 20260101000000Z -not_after 20460101000000Z -sha256 \
    -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -out "$work/root.pem"

key intermediate 2048
ext intermediate "basicConstraints=critical,CA:TRUE
keyUsage=critical,keyCertSign,cRLSign
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid"
cert intermediate root 20260101000000Z 20300101000000Z

key unit 3072
ext unit "basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature
extendedKeyUsage=critical,timeStamping
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid"
cert unit intermediate 20260101000000Z 20310101000000Z

key unit-responder 3072
ext unit-responder "basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature
extendedKeyUsage=OCSPSigning
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid"
cert unit-responder intermediate 20260101000000Z 20280101000000Z

key intermediate-responder 2048
ext intermediate-responder "basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature
extendedKeyUsage=OCSPSigning
subjectKeyIdentifier=hash
authorityKeyIdentifier=keyid"
cert intermediate-responder root 20260101000000Z 20360101000000Z

cat > "$work/tsa.cnf" <<EOF
[ tsa ]
default_tsa = tsa_config
[ tsa_config ]
serial = $work/tsa-serial
signer_digest = sha256
default_policy = 1.2.3.4.1
digests = sha256
ess_cert_id_alg = sha256
accuracy = secs:1
EOF
echo 01 > "$work/tsa-serial"
printf 'quorra token path fixture' > "$work/data"
openssl ts -query -data "$work/data" -sha256 -cert -no_nonce -out "$work/query.tsq"
openssl ts -reply -config "$work/tsa.cnf" -queryfile "$work/query.tsq" -signer "$work/unit.pem" \
    -inkey "$work/unit.key" -chain "$work/intermediate.pem" -token_out -out "$work/token.der"

# One "good" status: `openssl ocsp`'s index lists the certificate as valid.
status() { # certificate issuer responder out
    local serial expiry
    serial=$(openssl x509 -in "$work/$1.pem" -noout -serial | cut -d= -f2)
    expiry=$(openssl x509 -in "$work/$1.pem" -noout -enddate -dateopt iso_8601 | cut -d= -f2)
    printf 'V\t%s\t\t%s\tunknown\t/CN=%s\n' "$(date -u -d "$expiry" +%y%m%d%H%M%SZ)" "$serial" "$1" \
        > "$work/$4.index"
    openssl ocsp -issuer "$work/$2.pem" -cert "$work/$1.pem" -no_nonce -reqout "$work/$4.req"
    openssl ocsp -index "$work/$4.index" -rsigner "$work/$3.pem" -rkey "$work/$3.key" \
        -CA "$work/$2.pem" -reqin "$work/$4.req" -ndays 3650 -respout "$4.der"
}
status unit intermediate unit-responder unit-status
status intermediate root intermediate-responder intermediate-status

openssl x509 -in "$work/root.pem" -outform DER -out root.der
cp "$work/token.der" token.der
