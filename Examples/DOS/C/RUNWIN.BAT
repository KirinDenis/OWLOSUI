@echo off
rem Run one of this folder's programs in DOSBox-X, from Windows. Set
rem DOSBOXX to your dosbox-x.exe. The repository is drive C: in there, so
rem RUN.BAT finds the resident where it lives, in lib\dos.
rem
rem     RUNWIN HELLO          RUNWIN DEMO
rem     RUNWIN HELLO TEST     the test's keys typed by DOSBox-X, then it closes
rem
rem DOSBOXX_OPTS, if set, goes on the emulator's command line: for example
rem     set DOSBOXX_OPTS=-set output=surface
setlocal
cd /d %~dp0
if "%DOSBOXX%"=="" set DOSBOXX=C:\DOSFiles\dosbox-x\bin\x64\Release\dosbox-x.exe
if not exist "%DOSBOXX%" (
  echo dosbox-x.exe not found: set DOSBOXX to it.
  exit /b 1
)
if "%1"=="" (
  echo RUNWIN HELLO, or RUNWIN DEMO
  exit /b 1
)
for %%R in ("%~dp0..\..\..") do set ROOT=%%~fR
rem The clipboard API is how the toolkit shares Windows' clipboard.
"%DOSBOXX%" -noautoexec -set "dos clipboard api=true" %DOSBOXX_OPTS% -c "mount c %ROOT%" -c "c:" -c "cd \EXAMPLES\DOS\C" -c "RUN %1 %2" >nul 2>&1
