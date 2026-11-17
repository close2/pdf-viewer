#!/usr/bin/env python3
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.x509 import ocsp
from cryptography.x509.oid import NameOID

def utc(dt):
    return dt.astimezone(timezone.utc).replace(microsecond=0)

def build(cert_path, issuer_path, key_path, output_path, before_s, after_s):
    cert=x509.load_pem_x509_certificate(Path(cert_path).read_bytes())
    issuer=x509.load_pem_x509_certificate(Path(issuer_path).read_bytes())
    key=serialization.load_pem_private_key(Path(key_path).read_bytes(), password=None)
    now=utc(datetime.now(timezone.utc))
    response=(ocsp.OCSPResponseBuilder()
      .add_response(cert, issuer, hashes.SHA256(), ocsp.OCSPCertStatus.GOOD,
                    now-timedelta(seconds=int(before_s)), now+timedelta(seconds=int(after_s)), None, None)
      .responder_id(ocsp.OCSPResponderEncoding.HASH, issuer)
      .certificates([issuer])
      .sign(key, hashes.SHA256()))
    Path(output_path).write_bytes(response.public_bytes(serialization.Encoding.DER))
    print('producedAt', utc(response.produced_at_utc).isoformat())
    print('thisUpdate', utc(response.this_update_utc).isoformat())
    print('nextUpdate', utc(response.next_update_utc).isoformat())

def describe(path):
    r=ocsp.load_der_ocsp_response(Path(path).read_bytes())
    if r.response_status != ocsp.OCSPResponseStatus.SUCCESSFUL: raise ValueError(r.response_status)
    print('producedAt', utc(r.produced_at_utc).isoformat())
    print('thisUpdate', utc(r.this_update_utc).isoformat())
    print('nextUpdate', utc(r.next_update_utc).isoformat())
    print('status',r.certificate_status.name)

if __name__=='__main__':
    if sys.argv[1]=='build': build(*sys.argv[2:])
    elif sys.argv[1]=='describe': describe(sys.argv[2])
    else: raise SystemExit(2)
