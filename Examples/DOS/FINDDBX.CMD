@echo off
rem Find DOSBox-X for the launchers in the repository root, and say where
rem to get it if it is not there. Sets DOSBOXX for whoever called this:
rem
rem     call Examples\DOS\FINDDBX.CMD || exit /b 1
rem
rem Looked for, in order: DOSBOXX if it is already set, dosbox-x.exe on the
rem PATH, the folders its installer and its zip are usually put in.
if defined DOSBOXX if exist "%DOSBOXX%" exit /b 0
set DOSBOXX=
for /f "delims=" %%D in ('where dosbox-x.exe 2^>nul') do if not defined DOSBOXX set "DOSBOXX=%%D"
if defined DOSBOXX exit /b 0
for %%D in (
  "C:\DOSBox-X\dosbox-x.exe"
  "%ProgramFiles%\DOSBox-X\dosbox-x.exe"
  "%LOCALAPPDATA%\DOSBox-X\dosbox-x.exe"
  "C:\DOSFiles\dosbox-x\bin\x64\Release\dosbox-x.exe"
) do if not defined DOSBOXX if exist %%D set "DOSBOXX=%%~D"
if defined DOSBOXX exit /b 0
echo.
echo This example runs on DOS, in the DOSBox-X emulator, and DOSBox-X was
echo not found. Install it from https://dosbox-x.com - then either put
echo dosbox-x.exe on the PATH, or tell these launchers where it is:
echo.
echo     set DOSBOXX=C:\path\to\dosbox-x.exe
echo.
exit /b 1
