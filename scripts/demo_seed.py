#!/usr/bin/env python3
"""Seed data DEMO untuk presentasi SARPRAS & ALKES.

Dijalankan setelah `sarpras-seed` (master ruangan/item/config sudah ada):
    ./scripts/demo_seed.py
    BASE=http://127.0.0.1:8080 ./scripts/demo_seed.py

Membuat: pengguna demo, supplier + katalog, penawaran + penunjukan, permintaan
di berbagai status, PO (approve & kirim), penerimaan, serah terima, dan aset.

Idempoten secara praktis: data demo baru dibuat setiap kali dijalankan; jalankan
`reset` bila ingin membersihkan lebih dulu (lihat fungsi reset_demo).
"""

from __future__ import annotations

import json
import os
import subprocess
import time
import sys
import urllib.error
import urllib.request

BASE = os.environ.get("BASE", "http://127.0.0.1:8080")
DB = os.environ.get(
    "DATABASE_URL", "postgres://sarpras:sarpras_dev_pw@localhost:5432/sarpras_dev"
)
TAHUN = int(os.environ.get("TAHUN", "2026"))
PASSWORD_DEMO = "Demo1234"


def say(msg: str) -> None:
    print(f"\n\033[1;36m{msg}\033[0m")


def psql(sql: str) -> str:
    out = subprocess.run(
        ["psql", DB, "-tAc", sql], capture_output=True, text=True, check=True
    )
    return out.stdout.strip()


def api(method: str, path: str, token=None, body=None, _ulang: int = 0):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(BASE + path, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read() or b"{}")
    except urllib.error.HTTPError as e:
        payload = e.read().decode(errors="replace")
        try:
            hasil = json.loads(payload)
        except json.JSONDecodeError:
            return {"success": False, "error": {"code": str(e.code), "message": payload}}

        # Rate limiter API (100 req/menit/user) — skrip demo mengirim banyak
        # permintaan berurutan. Tunggu jendela berikutnya lalu ulangi.
        kode = (hasil.get("error") or {}).get("code")
        if kode in {"AUTH_005", "RATE_001"} and _ulang < 2:
            time.sleep(62)
            return api(method, path, token, body, _ulang + 1)
        return hasil


def login(username: str, password: str, _ulang: int = 0):
    hasil = api("POST", "/auth/login", body={"username": username, "password": password})
    kode = (hasil.get("error") or {}).get("code")
    if not hasil.get("success") and kode in {"AUTH_005", "RATE_001"} and _ulang < 2:
        time.sleep(62)
        return login(username, password, _ulang + 1)
    data = must(f"login {username}", hasil)
    return data["access_token"] if data else None


def must(label: str, result: dict):
    """Pastikan request sukses; cetak pesan singkat bila gagal."""
    if not result.get("success"):
        err = result.get("error") or {}
        print(f"   ⚠ {label}: {err.get('code')} — {err.get('message')}")
        return None
    return result.get("data")


def login(email: str, password: str):
    data = must("login", api("POST", "/auth/login", body={"email": email, "password": password}))
    return data["access_token"] if data else None


def reset_demo(admin: str) -> None:
    """Hapus data transaksi demo (master ruangan/item/config tetap)."""
    say("0. Bersihkan data demo lama")
    for path in ["/aset", "/serah-terima", "/penerimaan", "/po", "/penawaran", "/supplier"]:
        rows = api("GET", f"{path}?per_page=100", admin)
        if not rows.get("success"):
            continue
        for row in rows.get("data", []):
            rid = row.get("id")
            if rid is not None:
                api("DELETE", f"{path}/{rid}", admin)
    # Permintaan & turunannya lewat SQL (tidak ada endpoint hapus massal).
    psql(
        "DELETE FROM mutasi_aset; DELETE FROM aset_tetap; DELETE FROM serah_terima;"
        "DELETE FROM penerimaan; DELETE FROM po_detail; DELETE FROM po_header;"
        "DELETE FROM penawaran; DELETE FROM supplier_item; DELETE FROM supplier;"
        "DELETE FROM sla_tracking; DELETE FROM notifikasi; DELETE FROM approval_log;"
        "DELETE FROM disposisi;"
        "DELETE FROM permintaan; DELETE FROM audit_trail; DELETE FROM refresh_tokens;"
        "DELETE FROM config_snapshot;"
    )
    print("   data transaksi lama dibersihkan")


def main() -> int:
    say("1. Login administrator")
    admin = login("admin", "admin123")
    if not admin:
        print("Gagal login admin. Pastikan server berjalan dan seed master sudah dijalankan.")
        return 1

    if "--reset" in sys.argv:
        reset_demo(admin)

    say("2. Pengguna demo")
    ruangan_ids = [
        int(r)
        for r in psql(
            "SELECT id FROM ruangan WHERE is_active ORDER BY id LIMIT 5"
        ).split()
    ]
    if len(ruangan_ids) < 3:
        print("Butuh minimal 3 ruangan aktif di master.")
        return 1
    r_ugd, r_poli, r_lab = ruangan_ids[0], ruangan_ids[1], ruangan_ids[2]

    pengguna = [
        ("Mulyanti", "198803032010012011", "kasi@rsud-kendari.local", "kasi", "KASI_SARPRAS", None),
        ("dr. Ahmad Kabid", "198001012005011001", "kabid@rsud-kendari.local", "kabid", "KABID_YANMED", None),
        ("dr. Jeni Wadir", "197801252008032001", "wadir@rsud-kendari.local", "wadir", "WADIR_YANMED", None),
        ("Petugas Gudang", "199001012015011002", "gudang@rsud-kendari.local", "gudang", "PETUGAS_GUDANG", None),
        ("Admin Aset", "199202022018011003", "aset@rsud-kendari.local", "aset", "ADMIN_ASET", None),
        ("Perawat Ruangan", "199503032020011004", "ugd@rsud-kendari.local", "ugd", "USER_RUANGAN", r_ugd),
    ]
    for nama, nip, email, username, role, ruangan in pengguna:
        body = {
            "nama": nama,
            "nip": nip,
            "email": email,
            "username": username,
            "role": role,
            "ruangan_id": ruangan,
            "password": PASSWORD_DEMO,
        }
        hasil = api("POST", "/users", admin, body)
        if hasil.get("success"):
            continue
        # Sudah terdaftar → perbarui agar peran/ruangan sesuai skrip (idempoten).
        uid = psql(f"SELECT id FROM users WHERE email='{email}' OR nip='{nip}' LIMIT 1")
        if uid:
            perbarui = {k: v for k, v in body.items() if k != "password"}
            perbarui["is_active"] = True
            must(f"user {email} (perbarui)", api("PUT", f"/users/{int(uid)}", admin, perbarui))
        else:
            must(f"user {email}", hasil)
    print(f"   {len(pengguna)} pengguna demo (password: {PASSWORD_DEMO})")

    say("3. Supplier + katalog item")
    suppliers = []
    for nama, npwp, kontak, alamat in [
        ("PT Medika Jaya", "01.234.567.8-901.000", "0811-1111-111", "Jl. Ahmad Yani 12, Kendari"),
        ("CV Alkes Mandiri", "02.345.678.9-012.000", "0812-2222-222", "Jl. MT Haryono 45, Kendari"),
        ("PT Sarana Medik", "03.456.789.0-123.000", "0813-3333-333", "Jl. Diponegoro 8, Kendari"),
    ]:
        data = must(
            f"supplier {nama}",
            api("POST", "/supplier", admin, {"nama": nama, "npwp": npwp, "kontak": kontak, "alamat": alamat}),
        )
        if data:
            suppliers.append(data["id"])

    # Nama item dari Excel menyimpan spasi di ujung → gunakan TRIM.
    item_ac = int(psql("SELECT id FROM items WHERE TRIM(nama)='AC' AND is_active ORDER BY id LIMIT 1"))
    item_kursi = int(
        psql("SELECT id FROM items WHERE TRIM(nama) ILIKE 'Kursi Futura' AND is_active ORDER BY id LIMIT 1")
    )
    item_lemari = int(
        psql("SELECT id FROM items WHERE TRIM(nama) ILIKE 'Lemari' AND is_active ORDER BY id LIMIT 1")
    )
    satuan_unit = int(psql("SELECT id FROM satuan WHERE kode='UNIT'"))

    for sid, harga in zip(suppliers, [1_350_000, 1_250_000, 1_420_000]):
        must(
            "katalog",
            api("POST", f"/supplier/{sid}/items", admin, {"item_id": item_ac, "harga_referensi": harga}),
        )
    print(f"   {len(suppliers)} supplier + katalog item AC")

    say("4. Penawaran & penunjukan supplier")
    penawaran_ids = []
    for sid, harga in zip(suppliers, [1_350_000, 1_250_000, 1_420_000]):
        data = must(
            "penawaran",
            api(
                "POST",
                "/penawaran",
                admin,
                {
                    "supplier_id": sid,
                    "item_id": item_ac,
                    "harga_satuan": harga,
                    "sumber": "surat penawaran",
                },
            ),
        )
        if data:
            penawaran_ids.append(data["id"])
    if len(penawaran_ids) >= 2:
        must(
            "penunjukan",
            api("POST", f"/penawaran/{penawaran_ids[1]}/pilih", admin, {"alasan": "harga terendah & stok tersedia"}),
        )
    print("   3 penawaran (1.350.000 / 1.250.000 / 1.420.000) → dipilih CV Alkes Mandiri")

    # Penawaran + penunjukan untuk item kedua, agar PO item kursi lolos validasi
    # (F12: setiap item pada PO wajib punya minimal 3 penawaran, satu dipilih).
    item_kursi_id = int(
        psql("SELECT id FROM items WHERE TRIM(nama) ILIKE 'Kursi Futura' AND is_active ORDER BY id LIMIT 1")
    )
    p_kursi_ids = []
    for sid, harga in zip(suppliers, [1_150_000, 1_195_000, 1_240_000]):
        data = must(
            "penawaran kursi",
            api(
                "POST",
                "/penawaran",
                admin,
                {
                    "supplier_id": sid,
                    "item_id": item_kursi_id,
                    "harga_satuan": harga,
                    "sumber": "surat penawaran",
                },
            ),
        )
        if data:
            p_kursi_ids.append(data["id"])
    if p_kursi_ids:
        must(
            "penunjukan kursi",
            api("POST", f"/penawaran/{p_kursi_ids[0]}/pilih", admin, {"alasan": "harga terendah"}),
        )

    say("5. Permintaan di berbagai status")

    def buat_permintaan(ruangan: int, item: int, qty: int, ket: str = "kebutuhan operasional"):
        data = must(
            "permintaan",
            api(
                "POST",
                "/permintaan",
                admin,
                {"ruangan_id": ruangan, "items": [{"item_id": item, "jumlah": qty, "satuan_id": satuan_unit, "keterangan": ket}]},
            ),
        )
        return data["batch_id"] if data else None

    def submit(batch: str) -> None:
        must("submit", api("POST", f"/permintaan/batch/{batch}/submit", admin, {}))

    # a) Draft
    buat_permintaan(r_poli, item_kursi, 4, "penambahan kursi poli")
    # b) Submitted (menunggu Kasi)
    b_sub = buat_permintaan(r_lab, item_lemari, 2, "lemari penyimpanan reagen")
    if b_sub:
        submit(b_sub)
    # c) Disetujui Kasi (menunggu Kabid)
    b_kasi = buat_permintaan(r_ugd, item_kursi, 3)
    if b_kasi:
        submit(b_kasi)

    kasi = login("kasi", PASSWORD_DEMO)
    if kasi and b_kasi:
        must("setujui kasi", api("POST", f"/approval/{b_kasi}/setujui", kasi, {"catatan": "sesuai kebutuhan"}))
    # d) FINAL (3 jenjang)
    b_final = buat_permintaan(r_ugd, item_ac, 5, "pendingin ruang tindakan")
    if b_final:
        submit(b_final)
        if kasi:
            must("approve kasi", api("POST", f"/approval/{b_final}/setujui", kasi, {}))
        kabid = login("kabid", PASSWORD_DEMO)
        if kabid:
            must("approve kabid", api("POST", f"/approval/{b_final}/setujui", kabid, {}))
        wadir = login("wadir", PASSWORD_DEMO)
        if wadir:
            must("approve wadir", api("POST", f"/approval/{b_final}/setujui", wadir, {}))
    # e) RETURNED (ditolak)
    b_rej = buat_permintaan(r_poli, item_lemari, 1)
    if b_rej:
        submit(b_rej)
        if kasi:
            must(
                "tolak",
                api(
                    "POST",
                    f"/approval/{b_rej}/tolak",
                    kasi,
                    {"catatan": "spesifikasi belum lengkap, mohon dilengkapi"},
                ),
            )
    # f) FINAL kedua — sengaja dibiarkan agar menu "Final belum dipesan" pada
    #    beranda punya isi dan alur PO bisa dicoba dari browser.
    b_final2 = buat_permintaan(r_lab, item_kursi, 2, "kursi tunggu laboratorium")
    if b_final2:
        submit(b_final2)
        if kasi:
            must("approve kasi 2", api("POST", f"/approval/{b_final2}/setujui", kasi, {}))
        kabid2 = login("kabid", PASSWORD_DEMO)
        if kabid2:
            must("approve kabid 2", api("POST", f"/approval/{b_final2}/setujui", kabid2, {}))
        wadir2 = login("wadir", PASSWORD_DEMO)
        if wadir2:
            must("approve wadir 2", api("POST", f"/approval/{b_final2}/setujui", wadir2, {}))
    print("   draft · submitted · verifikasi kasi · FINAL · returned · FINAL (cadangan)")

    say("6. PO → approve → kirim")
    finals = [
        int(x)
        for x in psql(
            "SELECT id FROM permintaan WHERE status='FINAL' ORDER BY id"
        ).split()
    ]
    po = None
    if finals:
        po = must(
            "buat PO",
            api(
                "POST",
                "/po",
                admin,
                {
                    "supplier_id": suppliers[1],
                    "tahun": TAHUN,
                    "catatan": "pengadaan triwulan IV",
                    "details": [
                        {
                            "permintaan_id": finals[0],
                            "item_id": item_ac,
                            "qty": 5,
                            "satuan_id": satuan_unit,
                        }
                    ],
                },
            ),
        )
    if po:
        must("approve PO", api("POST", f"/po/{po['id']}/approve", admin, {}))
        must("kirim PO", api("POST", f"/po/{po['id']}/kirim", admin, {}))
        print(f"   {po['no_po']} → APPROVED → SENT")

    # PO kedua dibiarkan SENT agar menu Penerimaan punya pekerjaan nyata.
    if len(finals) > 1:
        po2 = must(
            "buat PO 2",
            api(
                "POST",
                "/po",
                admin,
                {
                    "supplier_id": suppliers[0],
                    "tahun": TAHUN,
                    "catatan": "pengadaan laboratorium (dalam perjalanan)",
                    "details": [
                        {
                            "permintaan_id": finals[1],
                            "item_id": item_kursi,
                            "qty": 2,
                            "satuan_id": satuan_unit,
                        }
                    ],
                },
            ),
        )
        if po2:
            must("approve PO 2", api("POST", f"/po/{po2['id']}/approve", admin, {}))
            must("kirim PO 2", api("POST", f"/po/{po2['id']}/kirim", admin, {}))
            print(f"   {po2['no_po']} → SENT (menunggu barang datang)")

    say("7. Penerimaan → serah terima → aset")
    if po:
        siap = must("siap terima", api("GET", f"/penerimaan/po/{po['id']}/siap", admin))
        if siap:
            detail_id = siap[0]["po_detail_id"]
            for qty, suffix in [(2, "001"), (3, "002")]:
                must(
                    "terima",
                    api(
                        "POST",
                        "/penerimaan",
                        admin,
                        {
                            "po_detail_id": detail_id,
                            "qty_terima": qty,
                            "no_do": f"DO/{TAHUN}/{suffix}",
                            "no_invoice": f"INV/{TAHUN}/{suffix}",
                            "no_faktur_pajak": f"FP/{TAHUN}/{suffix}",
                            "kondisi": "Baik",
                        },
                    ),
                )
            print("   penerimaan 2 + 3 → PO CLOSED")

    siap_serah = must("siap serah", api("GET", "/serah-terima/siap", admin))
    if siap_serah:
        st = must(
            "buat BAST",
            api(
                "POST",
                "/serah-terima",
                admin,
                {
                    "penerimaan_id": siap_serah[0]["penerimaan_id"],
                    "ruangan_id": r_ugd,
                    "penerima_ruangan_id": 1,
                    "catatan": "diserahkan ke ruangan",
                },
            ),
        )
        if st:
            must("konfirmasi", api("POST", f"/serah-terima/{st['id']}/konfirmasi", admin, {}))
            print(f"   {st['no_ba']} → CONFIRMED (aset otomatis dibuat)")

    say("8. Ringkasan data demo")
    for label, sql in [
        ("permintaan", "SELECT COUNT(*) FROM permintaan"),
        ("PO", "SELECT COUNT(*) FROM po_header"),
        ("penerimaan", "SELECT COUNT(*) FROM penerimaan"),
        ("serah terima", "SELECT COUNT(*) FROM serah_terima"),
        ("aset", "SELECT COUNT(*) FROM aset_tetap"),
        ("supplier", "SELECT COUNT(*) FROM supplier"),
        ("notifikasi", "SELECT COUNT(*) FROM notifikasi"),
        ("audit trail", "SELECT COUNT(*) FROM audit_trail"),
    ]:
        print(f"   {label:14}: {psql(sql)}")

    say("SELESAI — buka http://127.0.0.1:8080")
    print(
        f"""
Akun demo (password: {PASSWORD_DEMO}, kecuali admin):
  admin@rsud-kendari.local   Administrator  (admin123)
  kasi@rsud-kendari.local    Kasi Sarpras
  kabid@rsud-kendari.local   Kabid Yanmed
  wadir@rsud-kendari.local   Wadir Yanmed
  gudang@rsud-kendari.local  Petugas Gudang
  aset@rsud-kendari.local    Admin Aset
  user@rsud-kendari.local    User Ruangan

Alur yang bisa dicoba:
  1. ugd      → Permintaan → + Permintaan Baru → pilih item → Ajukan
  2. kasi     → Persetujuan → Setujui / Tolak (catatan wajib saat tolak)
  3. kabid / wadir → setujui hingga FINAL
  4. kasi     → Harga & Supplier → Bandingkan → Tunjuk supplier
  5. kasi     → Pesanan (PO) → + Buat PO → Setujui → Kirim
  6. admin@   → Penerimaan (via API) → Serah Terima → Aset
  7. admin@   → Dashboard Realisasi: nilai PO → terima → serah → aset
"""
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
