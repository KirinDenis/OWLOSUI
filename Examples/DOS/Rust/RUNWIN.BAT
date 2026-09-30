@echo off
rem Start DOSBox-X on this folder and run RUN.BAT inside it. Set DOSBOXX
rem to your dosbox-x.exe, or put it where the default says.
rem
rem     RUNWIN          the program, interactively
rem     RUNWIN SHOT     one frame into SHOT.BIN and SCREEN.BIN, then DOSBox-X closes
rem     RUNWIN QUIT     the smoke test: in and out
setlocal
cd /d %~dp0
if "%DOSBOXX%"=="" set DOSBOXX=C:\DOSFiles\dosbox-x\bin\x64\Release\dosbox-x.exe
if not exist "%DOSBOXX%" (
  echo dosbox-x.exe not found: set DOSBOXX to it.
  exit /b 1
)
if not "%1"=="" (
  del SHOT.BIN SCREEN.BIN 2>nul
  "%DOSBOXX%" -noautoexec -c "mount c %CD%" -c "c:" -c "RUN %1 EXIT"
) else (
  "%DOSBOXX%" -noautoexec -c "mount c %CD%" -c "c:" -c "RUN"
)
