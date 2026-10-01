@echo off
rem The C# demo in a window of its own instead of the console: the same
rem program with one line more. Alt+X leaves. Needs the .NET 8 SDK and Rust.
rem The program: Examples\Desktop\CSharp\06-Window
setlocal
cd /d "%~dp0"
where dotnet >nul 2>&1 || (echo This needs the .NET 8 SDK: https://dotnet.microsoft.com/download & pause & exit /b 1)
where cargo >nul 2>&1 || (echo This needs Rust: https://rustup.rs & pause & exit /b 1)
dotnet run --project Examples\Desktop\CSharp\06-Window
if errorlevel 1 pause
