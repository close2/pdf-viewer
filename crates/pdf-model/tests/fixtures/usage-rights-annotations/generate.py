#!/usr/bin/env python3
"""Generate signed, offline UR annotation-policy fixtures with OpenSSL and qpdf.

Use a scratch output directory. The fresh test key is deleted; dates and keys make
regeneration semantically equivalent rather than byte-identical.
"""
from pathlib import Path
import subprocess
import sys
import tempfile
import hashlib

OUT = Path(sys.argv[1]).resolve()
OUT.mkdir(parents=True, exist_ok=True)
ROOM = 8192
OLD = b'/ByteRange [0000000000 0000000000 0000000000 0000000000]'


def run(*args):
    subprocess.run(args, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def pdf(allowed):
    rights = '/Annots [/Create]' if allowed else '/Annots []'
    objects = [
        b'<< /Type /Catalog /Pages 2 0 R /Perms << /UR3 4 0 R >> >>',
        b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
        b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>',
        ('<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached '
         '/Name (Test-only UR policy fixture) /Reference [<< /Type /SigRef /TransformMethod /UR '
         '/TransformParams << /Type /TransformParams /V /2.2 /P true '
         f'/Document [/FullSave] /Form [/FillIn] {rights} >> >>] ').encode()
        + OLD + b' /Contents <' + b'0' * ROOM + b'> >>',
    ]
    data = bytearray(b'%PDF-1.7\n')
    offsets = []
    for n, body in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f'{n} 0 obj\n'.encode() + body + b'\nendobj\n')
    xref = len(data)
    data.extend(b'xref\n0 5\n0000000000 65535 f \n')
    for offset in offsets:
        data.extend(f'{offset:010d} 00000 n \n'.encode())
    data.extend(f'trailer\n<< /Size 5 /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <0123456789abcdef0123456789abcdef>] >>\nstartxref\n{xref}\n%%EOF\n'.encode())
    start = data.index(b'/Contents <') + len(b'/Contents ')
    end = data.index(b'>', start) + 1
    new = f'/ByteRange [0000000000 {start:010d} {end:010d} {len(data)-end:010d}]'.encode()
    assert len(new) == len(OLD)
    pos = data.index(OLD)
    data[pos:pos+len(OLD)] = new
    return data, start, end


with tempfile.TemporaryDirectory(prefix='ur-audit-') as temp:
    temp = Path(temp)
    key, cert = temp / 'key.pem', OUT / 'test-only-cert.pem'
    run('openssl', 'req', '-new', '-x509', '-newkey', 'rsa:2048', '-sha256', '-nodes', '-days', '3650',
        '-subj', '/CN=UR Annotation Audit Test Only', '-addext', 'basicConstraints=critical,CA:FALSE',
        '-addext', 'keyUsage=critical,digitalSignature', '-keyout', str(key), '-out', str(cert))
    for allowed in [True, False]:
        data, start, end = pdf(allowed)
        content, signature = temp / 'content.bin', temp / 'signature.der'
        content.write_bytes(data[:start] + data[end:])
        run('openssl', 'cms', '-sign', '-binary', '-md', 'sha256', '-in', str(content), '-signer', str(cert),
            '-inkey', str(key), '-outform', 'DER', '-out', str(signature))
        encoded = signature.read_bytes().hex().upper().encode()
        assert len(encoded) <= ROOM
        data[start+1:end-1] = encoded + b'0' * (ROOM-len(encoded))
        name = 'annotation-granted.pdf' if allowed else 'annotation-withheld.pdf'
        (OUT/name).write_bytes(data)
        run('openssl', 'cms', '-verify', '-binary', '-inform', 'DER', '-in', str(signature),
            '-content', str(content), '-noverify', '-out', '/dev/null')
        run('qpdf', '--check', str(OUT/name))
        print(name, len(data), hashlib.sha256(data).hexdigest())
