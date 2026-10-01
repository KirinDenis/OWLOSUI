@echo off
rem A two-panel file manager in C#, on this console, both sides on this
rem folder: Tab the other side, Insert marks, F3 view, F4 edit, F5 copy,
rem F6 move, F7 new folder, F8 delete, F10 quits. It works on real files.
rem Needs the .NET 8 SDK and Rust.
rem The program: Examples\Desktop\CSharp\03-Commander
setlocal
cd /d "%~dp0"
where dotnet >nul 2>&1 || (echo This needs the .NET 8 SDK: https://dotnet.microsoft.com/download & pause & exit /b 1)
where cargo >nul 2>&1 || (echo This needs Rust: https://rustup.rs & pause & exit /b 1)
dotnet run --project Examples\Desktop\CSharp\03-Commander
if errorlevel 1 pause
