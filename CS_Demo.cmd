@echo off
rem The C# demo in this console: a menu bar, an editor, a calculator, a
rem calendar, an ASCII table, a puzzle, a file viewer - everything the
rem toolkit has. Alt+X leaves. Needs the .NET 8 SDK and Rust: the C# build
rem compiles the toolkit's core with cargo the first time.
rem The program: Examples\Desktop\CSharp\05-OwlosDemo
setlocal
cd /d "%~dp0"
where dotnet >nul 2>&1 || (echo This needs the .NET 8 SDK: https://dotnet.microsoft.com/download & pause & exit /b 1)
where cargo >nul 2>&1 || (echo This needs Rust: https://rustup.rs & pause & exit /b 1)
dotnet run --project Examples\Desktop\CSharp\05-OwlosDemo
if errorlevel 1 pause
