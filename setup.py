import json
import subprocess
import os
import time

# ==========================================================
# KONFIGURASI - SILAKAN UBAH BAGIAN INI
# ==========================================================
# Token akun utama Anda yang memiliki hak akses admin/owner ke repositori.
MAIN_TOKEN = "ghp_naiijd"

# Username pemilik repositori.
OWNER_USERNAME = "Kyugito666"

# Nama repositori target.
REPO_NAME = "mawari-multi-wallet"
# ==========================================================

# Nama file untuk cache dan daftar username
TOKEN_CACHE_FILE = 'token_cache.json'
USERNAMES_FILE = 'usernames.txt'

def run_command(command, env=None):
    """Menjalankan perintah dan mengembalikan (sukses, output/error)."""
    try:
        result = subprocess.run(
            command,
            shell=True,
            check=True,
            capture_output=True,
            text=True,
            encoding='utf-8',
            env=env
        )
        return (True, result.stdout.strip())
    except subprocess.CalledProcessError as e:
        error_message = f"{e.stdout.strip()} {e.stderr.strip()}"
        return (False, error_message.strip())

def load_existing_data():
    """Memuat data dari file cache dan usernames.txt."""
    # Memuat cache token-username
    try:
        with open(TOKEN_CACHE_FILE, 'r') as f:
            token_cache = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        token_cache = {}

    # Memuat username yang sudah ada
    try:
        with open(USERNAMES_FILE, 'r') as f:
            # Menggunakan set untuk pencarian yang lebih cepat (O(1))
            existing_usernames = set(line.strip() for line in f if line.strip())
    except FileNotFoundError:
        existing_usernames = set()
        
    return token_cache, existing_usernames

def save_token_cache(cache):
    """Menyimpan cache token ke file JSON."""
    with open(TOKEN_CACHE_FILE, 'w') as f:
        json.dump(cache, f, indent=4)

def efficient_inviter():
    """
    Fungsi utama yang lebih efisien untuk mengambil username
    dan mengirimkan undangan kolaborasi.
    """
    print("🚀 Memulai proses otomatis yang efisien...")

    try:
        with open('tokens.json', 'r') as f:
            tokens = json.load(f)['tokens']
        print(f"   - ✅ Berhasil membaca {len(tokens)} token dari tokens.json.")
    except Exception as e:
        print(f"❌ FATAL: Gagal membaca tokens.json. Pastikan file ada dan formatnya benar. Error: {e}")
        return

    token_cache, existing_usernames = load_existing_data()
    print(f"   - ℹ️  Ditemukan {len(existing_usernames)} username yang sudah ada di {USERNAMES_FILE}.")
    
    # --- Langkah 1: Mengambil semua username dari token (dengan cache) ---
    all_usernames_from_tokens = []
    newly_discovered_usernames = []
    
    print("\n--- Tahap 1: Validasi Token dan Pengambilan Username ---")
    for index, token in enumerate(tokens):
        if token in token_cache:
            username = token_cache[token]
            print(f"   - ({index + 1}/{len(tokens)}) Mengambil dari cache: {username}")
            all_usernames_from_tokens.append(username)
            continue

        # Jika token tidak ada di cache, lakukan panggilan API
        print(f"   - ({index + 1}/{len(tokens)}) Memproses token baru via API...")
        env = os.environ.copy()
        env['GH_TOKEN'] = token
        success, username = run_command("gh api user --jq .login", env=env)
        
        if success:
            print(f"     ✅ Ditemukan username baru: {username}")
            token_cache[token] = username # Simpan ke cache
            all_usernames_from_tokens.append(username)
            if username not in existing_usernames:
                newly_discovered_usernames.append(username)
        else:
            print(f"     ⚠️  Gagal: Token tidak valid atau error API. Pesan: {username}")
        # Tidak perlu delay di sini karena kita meminimalkan panggilan API

    # Simpan cache yang sudah diperbarui
    save_token_cache(token_cache)
    print("\n   - ✅ Cache token-username telah diperbarui.")

    # --- Langkah 2: Menyimpan username baru ke file ---
    if newly_discovered_usernames:
        with open(USERNAMES_FILE, 'a') as f:
            for username in newly_discovered_usernames:
                f.write(f"{username}\n")
        print(f"   - ✅ {len(newly_discovered_usernames)} username baru berhasil ditambahkan ke {USERNAMES_FILE}.")
    else:
        print("   - ℹ️  Tidak ada username baru yang ditemukan.")

    # --- Langkah 3: Mengidentifikasi dan Mengirim undangan kolaborasi ---
    # Cari username yang ada di daftar token tapi belum ada di file usernames.txt awal
    usernames_to_invite = set(all_usernames_from_tokens) - existing_usernames
    
    print("\n--- Tahap 2: Mengirim Undangan Kolaborasi ---")
    if not usernames_to_invite:
        print("   - ✅ Semua username yang valid sudah menjadi kolaborator. Tidak ada undangan baru yang perlu dikirim.")
        print("\n✅ Semua proses telah selesai!")
        return
        
    print(f"   - ℹ️  Ditemukan {len(usernames_to_invite)} username baru untuk diundang.")
    env = os.environ.copy()
    env['GH_TOKEN'] = MAIN_TOKEN
    
    for username in usernames_to_invite:
        if username.lower() == OWNER_USERNAME.lower():
            print(f"   - ⏩ Melewati @{username} (pemilik repositori).")
            continue
            
        print(f"   - Mengirim undangan ke @{username}...")
        endpoint = f"repos/{OWNER_USERNAME}/{REPO_NAME}/collaborators/{username}"
        command = f"gh api --silent -X PUT -f permission='push' {endpoint}"
        success, result = run_command(command, env=env)
        
        if success:
             print("     ✅ Undangan berhasil dikirim!")
        else:
             print(f"     ⚠️  Gagal (mungkin sudah menjadi kolaborator atau username tidak ada). Pesan: {result}")
        
        # Jeda 2 detik antar undangan untuk menghindari rate limit
        time.sleep(2)

    print("\n✅ Semua proses telah selesai!")

# --- Jalankan fungsi utama saat skrip dieksekusi ---
if __name__ == "__main__":

    efficient_inviter()
