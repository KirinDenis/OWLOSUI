@echo off
rem Run the commander in DOSBox-X, from Windows. Set DOSBOXX to your
rem dosbox-x.exe. The repository is drive C: in there, so RUN.BAT finds the
rem resident where it lives, in lib\dos.
rem
rem     RUNWIN                   both sides on this folder
rem     RUNWIN C:\ C:\LIB        two folders, as DOS names them in there
rem     RUNWIN TEST              the test: TEST.CMD makes PLAY first
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
for %%R in ("%~dp0..\..\..") do set ROOT=%%~fR
rem The clipboard API is how the toolkit shares Windows' clipboard.
"%DOSBOXX%" -noautoexec -set "dos clipboard api=true" %DOSBOXX_OPTS% -c "mount c %ROOT%" -c "c:" -c "cd \EXAMPLES\DOS\COMMANDR" -c "RUN %1 %2" >nul 2>&1
