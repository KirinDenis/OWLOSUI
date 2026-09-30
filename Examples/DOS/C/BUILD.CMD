@echo off
rem The C examples, from Windows: DOSBox runs MAKE.BAT, where Open Watcom's
rem 16-bit DOS compiler makes HELLO.EXE and DEMO.EXE.
rem
rem   WATCOMDIR   the folder holding Watcom's BINW and H (the DOS-hosted
rem               tools). Watcom is free: open-watcom.github.io.
rem   DOSBOX      plain DOSBox 0.74. Not DOSBox-X: Watcom's compiler runs on
rem               the DOS/4GW extender, and that hangs in DOSBox-X here
rem               without a word. The programs themselves run in DOSBox-X
rem               (RUNWIN.BAT), whose DPMI the resident needs.
setlocal
cd /d %~dp0
if "%DOSBOX%"=="" set DOSBOX=C:\DOSBox-0.74-3\DOSBox.exe
if "%WATCOMDIR%"=="" set WATCOMDIR=C:\DOSFiles\watcom
if not exist "%WATCOMDIR%\BINW\WCL.EXE" (
  echo Watcom not found: set WATCOMDIR to the folder that holds BINW and H.
  exit /b 1
)
if not exist "%DOSBOX%" (
  echo DOSBox not found: set DOSBOX to DOSBox.exe.
  exit /b 1
)
for %%R in ("%~dp0..\..\..") do set ROOT=%%~fR
del MAKE.OUT 2>nul
"%DOSBOX%" -noautoexec -c "mount c %ROOT%" -c "mount w %WATCOMDIR%" -c "c:" -c "cd \EXAMPLES\DOS\C" -c "MAKE EXIT" >nul 2>&1
if not exist MAKE.OUT (
  echo MAKE.BAT never ran.
  exit /b 1
)
type MAKE.OUT
findstr /c:"not built" MAKE.OUT >nul && exit /b 1
exit /b 0
