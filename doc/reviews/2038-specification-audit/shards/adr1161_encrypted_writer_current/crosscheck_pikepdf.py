"""Cross-check the independent hash helper against qpdf via pikepdf, both passwords."""
import io
import pikepdf
from Crypto.Cipher import AES
from r6_writer_vector import alg2b

source = pikepdf.Pdf.new()
source.add_blank_page(page_size=(100, 100))
stream = io.BytesIO()
source.save(stream, encryption=pikepdf.Encryption(owner='keeper', user='reader', R=6))
stream.seek(0)
with pikepdf.open(stream, password='reader') as pdf:
    enc = pdf.trailer.Encrypt
    u, o = bytes(enc.U), bytes(enc.O)
    assert alg2b(b'reader', u[32:40], b'') == u[:32]
    assert alg2b(b'keeper', o[32:40], u) == o[:32]
    user_key = AES.new(alg2b(b'reader', u[40:48], b''), AES.MODE_CBC, bytes(16)).decrypt(bytes(enc.UE))
    owner_key = AES.new(alg2b(b'keeper', o[40:48], u), AES.MODE_CBC, bytes(16)).decrypt(bytes(enc.OE))
    assert user_key == owner_key
    perms = AES.new(user_key, AES.MODE_ECB).decrypt(bytes(enc.Perms))
    assert int.from_bytes(perms[:4], 'little', signed=True) == int(enc.P)
    assert perms[4:8] == bytes([255] * 4)
    assert perms[8:12] == b'Tadb'
print(f'pikepdf={pikepdf.__version__} qpdf={pikepdf.__libqpdf_version__}: both password hashes, both wrapped keys and consistent permissions PASS')
