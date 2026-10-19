@echo off
rem Runs CharPaper in an ordinary window instead of behind the desktop icons, for when the
rem wallpaper won't show. It stays a window until System > App > Show as switches it back.
rem Arguments are passed on, e.g.: run-windowed.bat --suite alice --language ru-RU

set "EXE=%~dp0charpaper.exe"
if not exist "%EXE%" (
    echo charpaper.exe is not next to this script: %~dp0
    pause
    exit /b 1
)

rem start returns at once, so this console closes instead of waiting for the app.
start "" "%EXE%" --windowed %*
