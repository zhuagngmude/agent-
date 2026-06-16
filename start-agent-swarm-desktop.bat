@echo off
setlocal

cd /d "%~dp0"

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\start-desktop-real-model.ps1"

if errorlevel 1 (
  echo.
  echo [desktop] Startup failed. Check the message above.
  pause
)
