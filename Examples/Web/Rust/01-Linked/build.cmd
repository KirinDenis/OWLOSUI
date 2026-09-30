@echo off
rem The Rust way into the browser: the core and the application linked into
rem one WebAssembly module, copied beside the page that loads it. Needs the
rem wasm32 target, once:
rem     rustup target add wasm32-unknown-unknown
rem Examples\Web\RUN.CMD linked builds this, serves it and opens it.
setlocal
cd /d %~dp0..\..\..\..
cargo build -p owlosui-wasm --target wasm32-unknown-unknown --release || exit /b 1
copy /y target\wasm32-unknown-unknown\release\owlosui_wasm.wasm Examples\Web\Rust\01-Linked\web\ >nul || exit /b 1
echo Built Examples\Web\Rust\01-Linked\web\owlosui_wasm.wasm. To see it: Examples\Web\RUN.CMD linked
