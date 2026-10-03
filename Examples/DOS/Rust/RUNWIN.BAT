@echo off
rem Start DOSBox-X and run RUN.BAT inside it. Set DOSBOXX to your
rem dosbox-x.exe, or put it where the default says. The repository is
rem drive C: in there, so the shared test keys in ..\TESTKEYS.BAT are
rem found too.
rem
rem     RUNWIN          the shared application, interactively
rem     RUNWIN HELLO    HELLO; RUNWIN DEMO, the demo
rem     RUNWIN DEMO TEST    the test's keys typed by DOSBox-X, then it closes
rem     RUNWIN SHOT     one frame into SHOT.BIN and SCREEN.BIN, then DOSBox-X closes
rem     RUNWIN QUIT     the smoke test: in and out
setlocal
cd /d %~dp0
if "%DOSBOXX%"=="" set DOSBOXX=C:\DOSFiles\dosbox-x\bin\x64\Release\dosbox-x.exe
if not exist "%DOSBOXX%" (
  echo dosbox-x.exe not found: set DOSBOXX to it.
  exit /b 1
)
for %%R in ("%~dp0..\..\..") do set ROOT=%%~fR
rem The clipboard API is how the toolkit shares Windows' clipboard.
set GO=-noautoexec -set "dos clipboard api=true" %DOSBOXX_OPTS% -c "mount c %ROOT%" -c "c:" -c "cd \EXAMPLES\DOS\RUST"
if /i "%1"=="HELLO" goto program
if /i "%1"=="DEMO" goto program
if "%1"=="" goto interactive
del SHOT.BIN SCREEN.BIN 2>nul
"%DOSBOXX%" %GO% -c "RUN %1 EXIT" >nul 2>&1
exit /b 0
:program
"%DOSBOXX%" %GO% -c "RUN %1 %2" >nul 2>&1
exit /b 0
:interactive
"%DOSBOXX%" %GO% -c "RUN" >nul 2>&1
