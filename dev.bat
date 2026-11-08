@echo off
rem Menjalankan backend (Rust API) dan frontend (Vite) sekaligus, masing-masing di jendela sendiri.
rem Tutup jendela masing-masing (atau tekan Ctrl+C di sana) untuk menghentikannya.
rem Prasyarat: PostgreSQL + PostGIS sudah jalan, dan file .env sudah diisi (lihat README).

setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
    echo [gagal] cargo tidak ditemukan. Pasang Rust dari https://rustup.rs
    exit /b 1
)

where bun >nul 2>nul
if errorlevel 1 (
    echo [gagal] bun tidak ditemukan. Pasang Bun dari https://bun.sh
    exit /b 1
)

rem Port backend. Default 8081 karena 8080 sering dipakai proyek lain. Ganti dengan:  set ARSGIS_PORT=9000
rem Nilai ini menimpa PORT di .env (variabel shell menang) dan diteruskan ke proxy Vite lewat API_PORT.
if not defined ARSGIS_PORT set "ARSGIS_PORT=8081"

netstat -ano | findstr /R /C:":%ARSGIS_PORT% .*LISTENING" >nul
if not errorlevel 1 (
    echo [gagal] Port %ARSGIS_PORT% sudah dipakai program lain.
    echo         Pakai port lain:  set ARSGIS_PORT=9000  lalu jalankan dev.bat lagi.
    exit /b 1
)

if not exist ".env" (
    echo [gagal] File .env belum ada. Salin dulu:  copy .env.example .env
    echo         lalu isi DATABASE_URL dan JWT_SECRET.
    exit /b 1
)

if not exist "web\node_modules" (
    echo Memasang dependensi frontend...
    pushd web
    call bun install
    if errorlevel 1 (
        popd
        echo [gagal] bun install gagal.
        exit /b 1
    )
    popd
)

echo Menjalankan backend  (cargo run -p pipa-api)  di jendela "arsgis-backend"
start "arsgis-backend" /d "%~dp0" cmd /k "set PORT=%ARSGIS_PORT%&& cargo run -p pipa-api"

echo Menjalankan frontend (bun run dev)            di jendela "arsgis-frontend"
start "arsgis-frontend" /d "%~dp0web" cmd /k "set API_PORT=%ARSGIS_PORT%&& bun run dev"

echo.
echo Buka http://localhost:5173  (backend di port %ARSGIS_PORT%)
echo Backend perlu beberapa saat untuk kompilasi pada jalan pertama.
endlocal
