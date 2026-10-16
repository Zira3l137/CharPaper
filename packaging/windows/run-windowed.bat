@echo off
rem Runs CharPaper in an ordinary window instead of behind the desktop icons.
rem Arguments are passed on, e.g.: run-windowed.bat --suite alice --language ru-RU

set "EXE=%~dp0charpaper.exe"
if not exist "%EXE%" (
    echo charpaper.exe is not next to this script: %~dp0
    pause
    exit /b 1
)

rem start returns at once, so this console closes instead of waiting for the app.
start "" "%EXE%" --windowed %*
