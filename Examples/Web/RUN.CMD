@echo off
rem The web examples: build the core for the browser, serve the repository
rem on localhost, open the page. Ctrl+C stops the server.
rem
rem     RUN.CMD              the demo, JavaScript step 5
rem     RUN.CMD 01-HelloWorld    any step of the JavaScript ladder by its folder
rem     RUN.CMD ladder       the list of all five
rem     RUN.CMD linked       the Rust program linked into WebAssembly
rem
rem Needs Rust and, once, the WebAssembly target:
rem     rustup target add wasm32-unknown-unknown
setlocal
cd /d "%~dp0..\.."

call lib\js\build.cmd || exit /b 1

set PAGE=/Examples/Web/JavaScript/%1/
if "%1"=="" set PAGE=/Examples/Web/JavaScript/05-Demo/
if /i "%1"=="ladder" set PAGE=/Examples/Web/JavaScript/
if /i not "%1"=="linked" goto serve
call Examples\Web\Rust\01-Linked\build.cmd >nul || exit /b 1
set PAGE=/Examples/Web/Rust/01-Linked/web/

:serve
cargo run -q --release -p owlosui-httpd -- . %PAGE%
