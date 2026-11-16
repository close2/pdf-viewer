#!/usr/bin/env python3
"""Offline exact-xref DocMDP + genuine RFC 3161 DTS fixtures; fresh disposable test credentials."""
import re, subprocess, sys
from pathlib import Path
if len(sys.argv) != 2:
    raise SystemExit("usage: generate_fixture.py OUTPUT_DIRECTORY (fresh disposable credentials)")
O=Path(sys.argv[1])
O.mkdir(parents=True,exist_ok=True); O=O.resolve()
ROOM=16384; OLD=b"/ByteRange [0000000000 0000000000 0000000000 0000000000]"
ID1=b"0123456789ABCDEF"; ID2_BASE=b"1111111111111111"; ID2_UPDATE=b"2222222222222222"
def run(*a): subprocess.run(a,check=True)
def br(pdf,after=0):
    pos=pdf.index(b"/Contents <",after)+len(b"/Contents "); end=pdf.index(b">",pos)+1
    vals=(0,pos,end,len(pdf)-end); i=pdf.index(OLD,after)
    new=("/ByteRange ["+" ".join(f"{x:010d}" for x in vals)+"]").encode()
    assert len(new)==len(OLD); pdf[i:i+len(OLD)]=new
def fill(pdf,der,after=0):
    pos=pdf.index(b"/Contents <",after)+len(b"/Contents "); end=pdf.index(b">",pos)+1
    h=der.hex().upper().encode(); n=end-pos-2; assert len(h)<=n
    pdf[pos+1:end-1]=h+b"0"*(n-len(h))
def data(pdf,after):
    pos=pdf.index(b"/Contents <",after)+len(b"/Contents "); end=pdf.index(b">",pos)+1
    return pdf[:pos]+pdf[end:]
def base():
    a=b"""%PDF-2.0
1 0 obj
<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Annots [5 0 R] >>
endobj
4 0 obj
<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached /Name (Quorra DocMDP Fixture Signer) /Reference [<< /Type /SigRef /TransformMethod /DocMDP /TransformParams << /Type /TransformParams /P 1 /V /1.2 >> >>] /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <"""
    b=b"""> >>
endobj
5 0 obj
<< /Type /Annot /Subtype /Widget /FT /Sig /T (Certification) /V 4 0 R /Rect [0 0 0 0] /F 4 /P 3 0 R >>
endobj
"""
    pdf=bytearray(a+b"0"*ROOM+b)
    offsets={int(m[1]):m.start() for m in re.finditer(rb"(?m)^(\d+) 0 obj$",pdf)}
    x=len(pdf); pdf+=b"xref\n0 6\n0000000000 65535 f \n"
    pdf+=b"".join(f"{offsets[i]:010d} 00000 n \n".encode() for i in range(1,6))
    pdf+=f"trailer\n<< /Size 6 /Root 1 0 R /ID [<{ID1.decode()}><{ID2_BASE.decode()}>] >>\nstartxref\n{x}\n%%EOF\n".encode()
    br(pdf); return pdf
def append(b,change):
    prev=int(list(re.finditer(rb"startxref\n(\d+)\n%%EOF",b))[-1][1])
    ds=bytearray(b)
    defs=[(1,b"<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> /AcroForm << /Fields [5 0 R 6 0 R] /SigFlags 3 >> /DSS 8 0 R >>")]
    media = b"201" if change else b"200"
    defs.append((3,b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 "+media+b" 200] /Resources << >> /Annots [5 0 R 6 0 R] >>"))
    signer=(O/"signer-cert.der").read_bytes(); tsa=(O/"tsa-cert.der").read_bytes()
    defs += [(6,b"<< /Type /Annot /Subtype /Widget /FT /Sig /T (DocumentTimestamp1) /V 7 0 R /Rect [0 0 0 0] /F 4 /P 3 0 R >>"),
      (7,b"<< /Type /DocTimeStamp /Filter /Adobe.PPKLite /SubFilter /ETSI.RFC3161 /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <"+b"0"*ROOM+b"> >>"),
      (8,b"<< /Type /DSS /Certs [9 0 R 10 0 R] >>"),
      (9,b"<< /Length "+str(len(signer)).encode()+b" >>\nstream\n"+signer+b"\nendstream"),
      (10,b"<< /Length "+str(len(tsa)).encode()+b" >>\nstream\n"+tsa+b"\nendstream")]
    offs=[]
    for n,obj in defs:
        ds+=b"\n"; offs.append((n,len(ds))); ds+=f"{n} 0 obj\n".encode()+obj+b"\nendobj\n"
    x=len(ds); ds+=b"xref\n"; i=0
    while i<len(offs):
        j=i+1
        while j<len(offs) and offs[j][0]==offs[j-1][0]+1: j+=1
        ds+=f"{offs[i][0]} {j-i}\n".encode()
        ds+=b"".join(f"{off:010d} 00000 n \n".encode() for _,off in offs[i:j]); i=j
    ds+=f"trailer\n<< /Size 11 /Root 1 0 R /Prev {prev} /ID [<{ID1.decode()}><{ID2_UPDATE.decode()}>] >>\nstartxref\n{x}\n%%EOF\n".encode()
    br(ds,len(b)); return ds

def main():
    run("openssl","req","-new","-x509","-newkey","rsa:2048","-sha256","-nodes","-days","3650",
      "-subj","/CN=Quorra DocMDP Fixture Signer/O=Quorra Test Only","-addext","basicConstraints=critical,CA:FALSE",
      "-addext","keyUsage=critical,digitalSignature","-keyout",str(O/"signer-key.pem"),"-out",str(O/"signer-cert.pem"))
    run("openssl","req","-new","-x509","-newkey","rsa:2048","-sha256","-nodes","-days","3650",
      "-subj","/CN=Quorra RFC3161 Test TSA/O=Quorra Test Only","-addext","basicConstraints=critical,CA:FALSE",
      "-addext","keyUsage=critical,digitalSignature","-addext","extendedKeyUsage=critical,timeStamping",
      "-keyout",str(O/"tsa-key.pem"),"-out",str(O/"tsa-cert.pem"))
    for x in ("signer","tsa"):
        run("openssl","x509","-in",str(O/f"{x}-cert.pem"),"-outform","DER","-out",str(O/f"{x}-cert.der"))
    initial=base(); (O/"signing-input.bin").write_bytes(data(initial,0))
    run("openssl","cms","-sign","-binary","-cades","-md","sha256","-in",str(O/"signing-input.bin"),
      "-signer",str(O/"signer-cert.pem"),"-inkey",str(O/"signer-key.pem"),"-outform","DER","-out",str(O/"signer-cms.der"))
    fill(initial,(O/"signer-cms.der").read_bytes()); (O/"base-signed.pdf").write_bytes(initial)
    (O/"tsa.cnf").write_text(f"""[ tsa ]
default_tsa = tsa_config1
[ tsa_config1 ]
serial = {O/'tsa-serial.txt'}
crypto_device = builtin
signer_cert = {O/'tsa-cert.pem'}
certs = {O/'tsa-cert.pem'}
signer_key = {O/'tsa-key.pem'}
signer_digest = sha256
default_policy = 1.3.6.1.4.1.55555.1
other_policies = 1.3.6.1.4.1.55555.2
digests = sha256, sha384, sha512
accuracy = secs:1
clock_precision_digits = 0
ordering = yes
tsa_name = yes
ess_cert_id_chain = no
ess_cert_id_alg = sha256
""")
    (O/"tsa-serial.txt").write_text("01\n")
    for name,change in (("dts-only",False),("dts-plus-page-change",True)):
        pdf=append(initial,change); (O/f"{name}-layout.pdf").write_bytes(pdf)
        content=data(pdf,len(initial)); (O/f"{name}-input.bin").write_bytes(content)
        run("openssl","ts","-query","-data",str(O/f"{name}-input.bin"),"-sha256","-cert","-no_nonce","-out",str(O/f"{name}.tsq"))
        run("openssl","ts","-reply","-token_out","-queryfile",str(O/f"{name}.tsq"),"-config",str(O/"tsa.cnf"),
          "-signer",str(O/"tsa-cert.pem"),"-inkey",str(O/"tsa-key.pem"),"-out",str(O/f"{name}-token.der"))
        fill(pdf,(O/f"{name}-token.der").read_bytes(),len(initial)); (O/f"{name}.pdf").write_bytes(pdf)
        run("openssl","ts","-verify","-token_in","-in",str(O/f"{name}-token.der"),"-data",str(O/f"{name}-input.bin"),"-CAfile",str(O/"tsa-cert.pem"))
        run("qpdf","--check",str(O/f"{name}.pdf"))
    run("openssl","cms","-verify","-binary","-inform","DER","-in",str(O/"signer-cms.der"),"-content",str(O/"signing-input.bin"),"-noverify","-out","/dev/null")
    run("qpdf","--check",str(O/"base-signed.pdf"))
    for name in ("signer","tsa"): run("openssl","x509","-in",str(O/f"{name}-cert.pem"),"-noout","-dates","-subject")
    run("openssl","x509","-in",str(O/"tsa-cert.pem"),"-noout","-ext","extendedKeyUsage")
    assert b"/MediaBox [0 0 200 200]" in (O/"dts-only.pdf").read_bytes()
    assert b"/MediaBox [0 0 201 200]" in (O/"dts-plus-page-change.pdf").read_bytes()
    print(f"Generated two valid exact-xref documents under {O}; the page coordinate edit itself is same-length.")
if __name__=="__main__": main()
