@echo off
setlocal
cd /d E:\workspace\desktop-app\auto-dark-mode\src-tauri
echo BUILD START > ..\build-out.txt
cargo build --release --features custom-protocol >> ..\build-out.txt 2>&1
if errorlevel 1 (
  echo BUILD FAILED >> ..\build-out.txt
  exit /b 1
)
echo BUILD OK >> ..\build-out.txt
taskkill /F /IM auto-dark-mode.exe >nul 2>&1
timeout /t 1 /nobreak >nul
copy /Y target\release\auto-dark-mode.exe "D:\Users\DanielHu\AppData\Local\AutoDarkMode\auto-dark-mode.exe" >> ..\build-out.txt 2>&1
if errorlevel 1 (
  echo COPY FAILED >> ..\build-out.txt
  exit /b 1
)
echo ALL DONE >> ..\build-out.txt
exit /b 0
