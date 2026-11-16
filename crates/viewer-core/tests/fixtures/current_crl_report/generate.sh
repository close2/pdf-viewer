#!/bin/sh
# Build offline, test-only signed PAdES inputs in a new output directory.
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 NEW_OUTPUT_DIRECTORY" >&2
  exit 2
fi
out=$1
if [ -e "$out" ]; then
  echo "refusing to use existing output directory: $out" >&2
  exit 2
fi
mkdir -p "$(dirname "$out")"
mkdir -- "$out"
out=$(cd "$out" && pwd)
script_dir=$(cd "$(dirname "$0")" && pwd)
workdir=$(mktemp -d "${TMPDIR:-/tmp}/signature-crl-public.XXXXXX")

cleanup() {
  rm -f "$workdir/root.key" "$workdir/leaf.key" "$workdir/root.csr" "$workdir/leaf.csr"
  rm -rf "$workdir"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
cd "$workdir"

# The generated fixed host instant is 30 minutes after construction time. Certificates are valid
# at that instant, with a CRL thisUpdate one minute on each side for the paired controls.
host_epoch=$(($(date -u +%s) + 1800))
host_time=$(date -u -d "@$host_epoch" '+%Y%m%d%H%M%SZ')
crl_before=$(date -u -d "@$((host_epoch - 60))" '+%Y%m%d%H%M%SZ')
crl_after=$(date -u -d "@$((host_epoch + 60))" '+%Y%m%d%H%M%SZ')
crl_next=$(date -u -d "@$((host_epoch + 60 * 60 * 24 * 60))" '+%Y%m%d%H%M%SZ')

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
[ signer_cert ]
basicConstraints = critical,CA:false
keyUsage = critical,digitalSignature
extendedKeyUsage = emailProtection
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer
EOF

openssl req -x509 -newkey rsa:2048 -nodes -sha256 \
  -keyout root.key -out root.pem -days 3650 -set_serial 1 \
  -subj '/CN=offline current-CRL test root' \
  -addext 'basicConstraints=critical,CA:TRUE,pathlen:0' \
  -addext 'keyUsage=critical,keyCertSign,cRLSign' \
  -addext 'subjectKeyIdentifier=hash'
openssl req -newkey rsa:2048 -nodes -sha256 \
  -keyout leaf.key -out leaf.csr -subj '/CN=offline current-CRL PAdES signer'
openssl ca -batch -config ca.cnf -in leaf.csr -out leaf.pem -notext \
  -extensions signer_cert -days 3650

openssl ca -batch -config ca.cnf -gencrl -crl_lastupdate "$crl_before" \
  -crl_nextupdate "$crl_next" -out current-crl.pem
openssl ca -batch -config ca.cnf -gencrl -crl_lastupdate "$crl_after" \
  -crl_nextupdate "$crl_next" -out future-crl.pem
openssl crl -in current-crl.pem -outform DER -out current-crl.der
openssl crl -in future-crl.pem -outform DER -out future-crl.der
openssl x509 -in root.pem -outform DER -out root.der

python3 "$script_dir/make_pades.py" layout current-crl.der current-layout.pdf current-input.bin
python3 "$script_dir/make_pades.py" layout future-crl.der future-layout.pdf future-input.bin
openssl cms -sign -binary -cades -md sha256 -in current-input.bin \
  -signer leaf.pem -inkey leaf.key -certfile root.pem -outform DER -out current-cms.der
openssl cms -sign -binary -cades -md sha256 -in future-input.bin \
  -signer leaf.pem -inkey leaf.key -certfile root.pem -outform DER -out future-cms.der
python3 "$script_dir/make_pades.py" finish current-layout.pdf current-cms.der current.pdf current-input.bin
python3 "$script_dir/make_pades.py" finish future-layout.pdf future-cms.der future.pdf future-input.bin

# Verify the actual detached ByteRanges, signer path at the fixed host instant, CRL signatures,
# and PDF syntax. qpdf is not treated as a semantic conformance oracle.
for kind in current future; do
  openssl cms -verify -binary -inform DER -in "$kind-cms.der" \
    -content "$kind-input.bin" -CAfile root.pem -purpose any -attime "$host_epoch" -out /dev/null
  openssl verify -attime "$host_epoch" -CAfile root.pem leaf.pem
  openssl crl -in "$kind-crl.pem" -noout -verify -CAfile root.pem -lastupdate -nextupdate
  qpdf --check "$kind.pdf"
done

cp root.der root.pem leaf.pem current-crl.der current-crl.pem future-crl.der future-crl.pem \
  current.pdf future.pdf "$out/"
printf '%s\n' "$host_epoch" > "$out/host-time-unix.txt"
printf '%s\n' "$host_time" > "$out/host-time-utc.txt"
openssl x509 -in leaf.pem -noout -dates -subject
sha256sum "$out/root.der" "$out/current-crl.der" "$out/future-crl.der" \
  "$out/current.pdf" "$out/future.pdf"
