@echo off
setlocal
cd /d "%~dp0"

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build.ps1"
set "EXIT_CODE=%ERRORLEVEL%"

echo.
if not "%EXIT_CODE%"=="0" (
    echo Build failed. Exit code: %EXIT_CODE%
) else (
    echo Build finished.
)

pause
exit /b %EXIT_CODE%
