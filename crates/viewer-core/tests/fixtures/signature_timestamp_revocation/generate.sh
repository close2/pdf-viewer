#!/bin/sh
set -eu

# Generate fresh public test material in a new scratch directory supplied by the caller.
# Private keys are ephemeral and removed even if a verification step fails.
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
: "${1:?usage: generate.sh NEW_OUTPUT_DIRECTORY}"
mkdir -- "$1"
cd -- "$1"
trap 'rm -f root.key tsa.key doc.key' EXIT HUP INT TERM
mkdir ca-db
: > ca-db/index.txt
printf '1000\n' > ca-db/serial
printf '1000\n' > ca-db/crlnumber

cat > ca.cnf <<'EOF'
[ ca ]
default_ca = local_ca
[ local_ca ]
dir = ./ca-db
database = $dir/index.txt
new_certs_dir = $dir
certificate = ./root.pem
private_key = ./root.key
serial = $dir/serial
crlnumber = $dir/crlnumber
default_md = sha256
default_days = 3650
default_crl_days = 60
policy = policy
copy_extensions = none
unique_subject = no
[ policy ]
commonName = supplied
[ tsa_cert ]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = critical,timeStamping
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer
[ doc_cert ]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = emailProtection
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer
EOF

openssl req -x509 -newkey rsa:2048 -nodes -sha256 \
  -keyout root.key -out root.pem -days 3650 -set_serial 1 \
  -subj '/CN=offline revoked TSA test root' \
  -addext 'basicConstraints=critical,CA:TRUE' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' \
  -addext 'subjectKeyIdentifier=hash'
openssl req -newkey rsa:2048 -nodes -sha256 \
  -keyout tsa.key -out tsa.csr -subj '/CN=offline timestamp authority'
openssl ca -batch -config ca.cnf -in tsa.csr -out tsa.pem -notext \
  -extensions tsa_cert -days 3650
openssl req -newkey rsa:2048 -nodes -sha256 \
  -keyout doc.key -out doc.csr -subj '/CN=offline document signer'
openssl ca -batch -config ca.cnf -in doc.csr -out doc.pem -notext \
  -extensions doc_cert -days 3650

# Produce a genuinely signed detached CMS value first. The timestamp is later computed over its
# actual SignerInfo.signature octets, then added only as an unsigned attribute (which cannot alter
# the existing signed attributes or signature).
printf 'offline signed document bytes\n' > signed-content.bin
openssl cms -sign -binary -in signed-content.bin -signer doc.pem -inkey doc.key \
  -outform DER -out base-signed.cms -md sha256 -nosmimecap -cades
python3 "$script_dir/make_cms_fixture.py" extract-signature base-signed.cms signature.bin

cat > tsa.cnf <<'EOF'
[ tsa ]
default_tsa = tsa_config1
[ tsa_config1 ]
dir = .
serial = $dir/tsa_serial
crypto_device = builtin
signer_cert = $dir/tsa.pem
certs = $dir/root.pem
signer_key = $dir/tsa.key
signer_digest = sha256
default_policy = 1.2.3.4.1
other_policies = 1.2.3.4.2
digests = sha256
accuracy = secs:1, millisecs:500, microsecs:100
ordering = yes
tsa_name = yes
ess_cert_id_chain = no
ess_cert_id_alg = sha256
EOF
printf '01\n' > tsa_serial
openssl ts -query -data signature.bin -sha256 -cert -out request.tsq
openssl ts -reply -config tsa.cnf -queryfile request.tsq -out token.tsr
openssl ts -reply -in token.tsr -token_out -out token.der

python3 "$script_dir/make_cms_fixture.py" attach-token token.der base-signed.cms \
  cms-with-signature-timestamp.der

# A clean, correctly signed CRL is the positive control: it covers the TSA serial without listing
# it. Capture it before revoking the TSA certificate.
openssl ca -batch -config ca.cnf -gencrl -out tsa-good-crl.pem
openssl crl -in tsa-good-crl.pem -outform DER -out tsa-good-crl.der

# The CRL is emitted after the TSA certificate is issued and marks that exact TSA serial
# keyCompromise. This deliberately gives a cryptographically genuine, current revoked-path input.
openssl ca -batch -config ca.cnf -revoke tsa.pem -crl_reason keyCompromise
openssl ca -batch -config ca.cnf -gencrl -out tsa-revoked-crl.pem
openssl crl -in tsa-revoked-crl.pem -outform DER -out tsa-revoked-crl.der
openssl x509 -in root.pem -outform DER -out root.der
openssl x509 -in tsa.pem -outform DER -out tsa.der
openssl x509 -in doc.pem -outform DER -out doc.der

# Independent controls: token verifies under included signer; TSA chain verifies to the local
# root; CRL signature is by that root and identifies the TSA serial as keyCompromise.
openssl ts -verify -in token.tsr -CAfile root.pem -untrusted tsa.pem -queryfile request.tsq
openssl verify -CAfile root.pem tsa.pem
openssl verify -CAfile root.pem doc.pem
openssl cms -verify -binary -inform DER -in cms-with-signature-timestamp.der \
  -content signed-content.bin -CAfile root.pem -purpose any -out /dev/null
openssl crl -in tsa-good-crl.pem -noout -verify -CAfile root.pem
openssl crl -in tsa-revoked-crl.pem -noout -verify -CAfile root.pem -issuer -lastupdate -nextupdate
openssl crl -in tsa-revoked-crl.pem -noout -text | sed -n '/Revoked Certificates:/,/Signature Algorithm/p'
sha256sum root.der tsa.der doc.der tsa-good-crl.der tsa-revoked-crl.der token.der cms-with-signature-timestamp.der

# These generated private keys are throwaway local fixture material, not credentials. Remove them
# after the public signed artifacts and independent verification have been produced.
rm -f root.key tsa.key doc.key tsa.csr doc.csr
