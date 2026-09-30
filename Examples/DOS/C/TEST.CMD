@echo off
rem The C examples, tested: built, then HELLO and DEMO run under the
rem resident in DOSBox-X, typed at, photographed, and read back
rem (..\CHECK.CMD). The resident itself must be built: lib\dos\build.cmd.
setlocal
cd /d %~dp0
call .\BUILD.CMD || exit /b 1
call ..\CHECK.CMD
