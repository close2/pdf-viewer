#!/bin/sh
# Rebuild a semantically equivalent, test-only PAdES PDF, entirely offline.
# Usage: ./generate.sh [output-directory]. A fresh RSA private key is created in that
# directory and must never be used for real signing. Regeneration is not byte-identical:
# the certificate and CMS signingTime change. The checked-in fixture remains the test input.
set -eu
out=${1:-"$(dirname "$0")/generated"}
mkdir -p "$out"
out=$(cd "$out" && pwd)

openssl req -new -x509 -newkey rsa:2048 -sha256 -nodes -days 3650 \
  -subj '/CN=Quorra PAdES Test Fixture/O=Quorra Test Only' \
  -keyout "$out/pades-test-key.pem" -out "$out/pades-test-cert.pem"
openssl x509 -in "$out/pades-test-cert.pem" -outform DER -out "$out/pades-test-cert.der"

python3 - "$out" <<'PY'
import re, sys
from pathlib import Path
p=Path(sys.argv[1])
hex_room=16384
prefix=b'''%PDF-2.0\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Annots [5 0 R] >>\nendobj\n4 0 obj\n<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached /Name (Quorra PAdES Fixture Signer) /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <'''
suffix=b'''> >>\nendobj\n5 0 obj\n<< /FT /Sig /T (PadesFixture) /V 4 0 R /Subtype /Widget /Rect [20 20 180 60] /P 3 0 R >>\nendobj\n'''
pdf=bytearray(prefix+b'0'*hex_room+suffix)
starts={int(m.group(1)):m.start() for m in re.finditer(rb'(?m)^(\d+) 0 obj$',pdf)}
xref_at=len(pdf)
pdf.extend(b'xref\n0 6\n0000000000 65535 f \n'+b''.join(f'{starts[i]:010d} 00000 n \n'.encode() for i in range(1,6)))
pdf.extend(f'trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n'.encode())
start=pdf.index(b'/Contents <')+len(b'/Contents ')
end=pdf.index(b'>',start)+1
ranges=(0,start,end,len(pdf)-end)
old=b'/ByteRange [0000000000 0000000000 0000000000 0000000000]'
new=('/ByteRange ['+' '.join(f'{x:010d}' for x in ranges)+']').encode()
assert len(old)==len(new)
at=pdf.index(old)
pdf[at:at+len(old)]=new
(p/'pades-signing-input.bin').write_bytes(pdf[:start]+pdf[end:])
(p/'pades-layout-placeholder.pdf').write_bytes(pdf)
(p/'pades-ranges.txt').write_text(' '.join(map(str,ranges))+'\n')
PY

openssl cms -sign -binary -cades -md sha256 \
  -in "$out/pades-signing-input.bin" \
  -signer "$out/pades-test-cert.pem" -inkey "$out/pades-test-key.pem" \
  -outform DER -out "$out/pades-cms.der"
python3 - "$out" <<'PY'
import sys
from pathlib import Path
p=Path(sys.argv[1])
pdf=bytearray((p/'pades-layout-placeholder.pdf').read_bytes())
der=(p/'pades-cms.der').read_bytes()
start=pdf.index(b'/Contents <')+len(b'/Contents ')
end=pdf.index(b'>',start)+1
encoded=der.hex().upper().encode()
room=end-start-2
assert len(encoded)<=room
pdf[start+1:end-1]=encoded+b'0'*(room-len(encoded))
(p/'pades-valid.pdf').write_bytes(pdf)
tampered=bytearray(pdf)
old=b'/MediaBox [0 0 200 200]'
at=tampered.index(old)
tampered[at:at+len(old)]=b'/MediaBox [0 0 201 200]'
(p/'pades-tampered.pdf').write_bytes(tampered)
(p/'pades-tampered-input.bin').write_bytes(tampered[:start]+tampered[end:])
PY

openssl cms -verify -binary -inform DER -in "$out/pades-cms.der" \
  -content "$out/pades-signing-input.bin" -noverify -out /dev/null
if openssl cms -verify -binary -inform DER -in "$out/pades-cms.der" \
  -content "$out/pades-tampered-input.bin" -noverify -out /dev/null 2>/dev/null; then
  echo 'ERROR: tampered same-ByteRange control unexpectedly verified' >&2
  exit 1
fi
qpdf --check "$out/pades-valid.pdf"
qpdf --check "$out/pades-tampered.pdf"
openssl x509 -in "$out/pades-test-cert.pem" -noout -dates -subject
python3 - "$out/pades-test-cert.pem" <<'PY'
import datetime, subprocess, sys
raw=subprocess.check_output(['openssl','x509','-in',sys.argv[1],'-noout','-startdate'],text=True)
start=datetime.datetime.strptime(raw.strip().split('=',1)[1],'%b %d %H:%M:%S %Y GMT').replace(tzinfo=datetime.timezone.utc)
print('Use this fixed verifier instant for this generated cert:',int((start+datetime.timedelta(hours=1)).timestamp()))
PY
