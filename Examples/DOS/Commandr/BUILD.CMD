@echo off
rem The commander, from Windows: DOSBox runs MAKE.BAT, where Borland Pascal
rem 7's command-line compiler makes COMMANDR.EXE.
rem
rem   BPDIR    the folder holding Borland's BIN\BPC.EXE. Borland Pascal is
rem            commercial and is not in this repository.
rem   DOSBOX   plain DOSBox 0.74, as for the other examples: the compiler
rem            runs there, the program in DOSBox-X (RUNWIN.BAT).
setlocal
cd /d %~dp0
if "%DOSBOX%"=="" set DOSBOX=C:\DOSBox-0.74-3\DOSBox.exe
if "%BPDIR%"=="" set BPDIR=C:\DOSFiles\BP
if not exist "%BPDIR%\BIN\BPC.EXE" (
  echo Borland Pascal not found: set BPDIR to the folder that holds BIN\BPC.EXE.
  exit /b 1
)
if not exist "%DOSBOX%" (
  echo DOSBox not found: set DOSBOX to DOSBox.exe.
  exit /b 1
)
for %%R in ("%~dp0..\..\..") do set ROOT=%%~fR
del MAKE.OUT 2>nul
"%DOSBOX%" -noautoexec -c "mount c %ROOT%" -c "mount p %BPDIR%" -c "c:" -c "cd \EXAMPLES\DOS\COMMANDR" -c "MAKE EXIT" >nul 2>&1
if not exist MAKE.OUT (
  echo MAKE.BAT never ran.
  exit /b 1
)
type MAKE.OUT
findstr /c:"not built" MAKE.OUT >nul && exit /b 1
exit /b 0
