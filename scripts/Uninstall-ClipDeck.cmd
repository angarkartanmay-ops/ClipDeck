@echo off
rem Removes ClipDeck (any version) that was installed for this Windows account.
rem Double-click to run. No administrator rights needed.
rem Installed with the per-machine .msi instead? Use Settings > Apps > Installed apps.
setlocal EnableExtensions
title Uninstall ClipDeck

set "APPDIR=%LOCALAPPDATA%\ClipDeck"
set "UNINST=%APPDIR%\uninstall.exe"
set "UNKEY=HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ClipDeck"
set "RUNKEY=HKCU\Software\Microsoft\Windows\CurrentVersion\Run"

echo.
echo   Uninstall ClipDeck
echo   ------------------
echo.

set "VER="
for /f "tokens=2,*" %%A in ('reg query "%UNKEY%" /v DisplayVersion 2^>nul ^| find "DisplayVersion"') do set "VER=%%B"

if not defined VER if not exist "%UNINST%" (
  echo   ClipDeck does not seem to be installed for this account.
  goto cleanup
)

if defined VER (
  echo   Found ClipDeck %VER% in %APPDIR%
) else (
  echo   Found a ClipDeck uninstaller in %APPDIR%
)

echo.
echo   [1/3] Closing ClipDeck...
taskkill /IM clipdeck.exe /F >nul 2>&1
timeout /t 1 /nobreak >nul

echo   [2/3] Running the ClipDeck uninstaller...
if exist "%UNINST%" (
  rem "_?=" makes the NSIS uninstaller run in place and wait until it is done.
  "%UNINST%" /S _?=%APPDIR%
  del /F /Q "%UNINST%" >nul 2>&1
  rmdir "%APPDIR%" >nul 2>&1
) else (
  echo         No uninstaller found; removing the leftover entry only.
  reg delete "%UNKEY%" /f >nul 2>&1
)

:cleanup
echo   [3/3] Removing the start-with-Windows entry...
reg delete "%RUNKEY%" /v ClipDeck /f >nul 2>&1

echo.
echo   Your clipboard history and settings are in %APPDATA%\ClipDeck.
echo   A new install reuses them, so keeping them is usually what you want.
choice /C KD /N /M "  [K]eep them or [D]elete them? "
if errorlevel 2 (
  rmdir /S /Q "%APPDATA%\ClipDeck" >nul 2>&1
  rmdir /S /Q "%LOCALAPPDATA%\com.tanmayangarkar.clipdeck" >nul 2>&1
  echo   History, settings and the web view cache were deleted.
) else (
  echo   Kept.
)

echo.
reg query "%UNKEY%" >nul 2>&1
if errorlevel 1 (
  if exist "%APPDIR%\clipdeck.exe" (
    echo   Something is left in %APPDIR%. Close ClipDeck from the tray and run this again.
  ) else (
    echo   Done. ClipDeck is removed.
  )
) else (
  echo   The uninstall entry is still registered. Remove ClipDeck from
  echo   Settings ^> Apps ^> Installed apps, then run this again.
)

echo.
pause
