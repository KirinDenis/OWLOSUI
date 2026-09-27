@echo off
rem Step 3, the browser: the core and the application in one WebAssembly
rem module, copied beside the page that loads it. Needs the wasm32 target,
rem once:
rem     rustup target add wasm32-unknown-unknown
setlocal
cd /d %~dp0..\..\..
cargo build -p owlosui-wasm --target wasm32-unknown-unknown --release || exit /b 1
copy /y target\wasm32-unknown-unknown\release\owlosui_wasm.wasm Examples\Rust\03-Browser\web\ >nul || exit /b 1
echo Built Examples\Rust\03-Browser\web\owlosui_wasm.wasm. Serve that folder with any static server:
echo     python -m http.server 8765 --directory Examples\Rust\03-Browser\web
echo and open http://localhost:8765
