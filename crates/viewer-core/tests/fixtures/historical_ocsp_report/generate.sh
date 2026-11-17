#!/bin/sh
set -eu
if [ "$#" -ne 1 ]; then
  echo 'usage: build_public_fixture.sh NEW_OUTPUT_DIRECTORY' >&2
  exit 2
fi
case "$1" in /*) out=$1 ;; *) out=$(pwd)/$1 ;; esac
if [ -e "$out" ]; then
  echo "refusing to overwrite existing output: $out" >&2
  exit 2
fi
mkdir -m 700 "$out"
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
work=$(mktemp -d)
cleanup() { rm -rf "$work"; }
trap cleanup EXIT HUP INT TERM
cd "$work"
mkdir ca-db; : > ca-db/index.txt
printf '1000\n' > ca-db/serial
printf '1000\n' > ca-db/crlnumber
cat > ca.cnf <<'CNF'
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
CNF
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -keyout root.key -out root.pem -days 3650 -set_serial 1 -subj '/CN=offline historical OCSP test root' -addext 'basicConstraints=critical,CA:TRUE' -addext 'keyUsage=critical,keyCertSign,cRLSign,digitalSignature' -addext 'subjectKeyIdentifier=hash'
openssl req -newkey rsa:2048 -nodes -sha256 -keyout tsa.key -out tsa.csr -subj '/CN=offline timestamp authority'
openssl ca -batch -config ca.cnf -in tsa.csr -out tsa.pem -notext -extensions tsa_cert -days 3650
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -keyout doc-ca.key -out doc-ca.pem -days 3650 -set_serial 2 -subj '/CN=offline document CA anchor' -addext 'basicConstraints=critical,CA:TRUE' -addext 'keyUsage=critical,keyCertSign,cRLSign,digitalSignature' -addext 'subjectKeyIdentifier=hash'
openssl req -newkey rsa:2048 -nodes -sha256 -keyout doc.key -out doc.csr -subj '/CN=offline historical OCSP signer'
openssl x509 -req -in doc.csr -CA doc-ca.pem -CAkey doc-ca.key -CAcreateserial -out doc.pem -days 3650 -sha256 -extfile ca.cnf -extensions doc_cert
openssl ca -batch -config ca.cnf -gencrl -out tsa-good-crl.pem
openssl crl -in tsa-good-crl.pem -outform DER -out tsa-good-crl.der
python3 "$script_dir/make_ocsp.py" build doc.pem doc-ca.pem doc-ca.key early.ocsp.der 600 3600
python3 "$script_dir/make_ocsp.py" describe early.ocsp.der > early-times.txt
sleep 2
python3 "$script_dir/make_pades.py" layout tsa-good-crl.der signature-layout.pdf signed-content.bin
openssl cms -sign -binary -in signed-content.bin -signer doc.pem -inkey doc.key -outform DER -out base-signed.cms -md sha256 -nosmimecap -cades
python3 "$script_dir/make_cms_fixture.py" extract-signature base-signed.cms signature.bin
cat > tsa.cnf <<'TSA'
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
TSA
printf '01\n' > tsa_serial
openssl ts -query -data signature.bin -sha256 -cert -out request.tsq
openssl ts -reply -config tsa.cnf -queryfile request.tsq -out token.tsr
openssl ts -reply -in token.tsr -token_out -out token.der
openssl ts -reply -in token.tsr -text > token-info.txt
python3 "$script_dir/make_cms_fixture.py" attach-token token.der base-signed.cms cms-with-signature-timestamp.der
sleep 2
python3 "$script_dir/make_pades.py" finish signature-layout.pdf cms-with-signature-timestamp.der base.pdf signed-content-filled.bin
python3 "$script_dir/make_ocsp.py" build doc.pem doc-ca.pem doc-ca.key late.ocsp.der 600 3600
python3 "$script_dir/make_ocsp.py" describe late.ocsp.der > late-times.txt
python3 "$script_dir/append_dss.py" base.pdf early.ocsp.der tsa-good-crl.der early.pdf
python3 "$script_dir/append_dss.py" base.pdf late.ocsp.der tsa-good-crl.der late.pdf
openssl ts -verify -in token.tsr -CAfile root.pem -untrusted tsa.pem -queryfile request.tsq
openssl cms -verify -binary -inform DER -in cms-with-signature-timestamp.der -content signed-content.bin -noverify -out /dev/null
openssl cms -verify -binary -inform DER -in cms-with-signature-timestamp.der -content signed-content-filled.bin -noverify -out /dev/null
openssl verify -CAfile doc-ca.pem doc.pem
openssl verify -CAfile root.pem tsa.pem
openssl crl -in tsa-good-crl.pem -noout -verify -CAfile root.pem
openssl ocsp -issuer doc-ca.pem -sha256 -cert doc.pem -reqout doc-request.der -no_nonce
openssl ocsp -reqin doc-request.der -respin early.ocsp.der -CAfile doc-ca.pem -no_nonce -text > early-ocsp-verify.txt
openssl ocsp -reqin doc-request.der -respin late.ocsp.der -CAfile doc-ca.pem -no_nonce -text > late-ocsp-verify.txt
python3 "$script_dir/verify_times.py" token-info.txt early.ocsp.der late.ocsp.der
trusted_time=$(cat trusted-time-unix.txt)
openssl verify -attime "$trusted_time" -CAfile doc-ca.pem doc.pem
openssl verify -attime "$trusted_time" -CAfile root.pem tsa.pem
python3 - "$work" "$out" <<'PY'
from pathlib import Path
import sys, shutil
src=Path(sys.argv[1]); out=Path(sys.argv[2])
for name in ['early.pdf','late.pdf','root.pem','root.der','host-time-unix.txt','token-info.txt','trusted-time-unix.txt','early-times.txt','late-times.txt','early-ocsp-verify.txt','late-ocsp-verify.txt','early.ocsp.der','late.ocsp.der','tsa-good-crl.der','doc.pem','doc-ca.pem','doc-ca.der','tsa.pem']:
    p=src/name
    if name=='root.der':
        pass
    elif name=='host-time-unix.txt':
        import time
        (out/name).write_text(str(int(time.time()))+'\n')
    elif p.exists(): shutil.copyfile(p,out/name)
PY
openssl x509 -in root.pem -outform DER -out "$out/root.der"
openssl x509 -in doc-ca.pem -outform DER -out "$out/doc-ca.der"
if command -v qpdf >/dev/null 2>&1; then qpdf --check "$out/early.pdf"; qpdf --check "$out/late.pdf"; fi
cd "$out"
sha256sum *.pdf *.der > SHA256SUMS
# Keep private test keys and intermediate CMS/OCSP request material only in the trapped temp dir.
