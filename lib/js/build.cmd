@echo off
rem The core for JavaScript: lib/serve compiled into owlosui-wire.wasm,
rem beside owlosui.js, which loads it from there. Needs the wasm32
rem target, once:
rem     rustup target add wasm32-unknown-unknown
setlocal
cd /d %~dp0..\..
cargo build -p owlosui-wire --target wasm32-unknown-unknown --release || exit /b 1
copy /y target\wasm32-unknown-unknown\release\owlosui_wire.wasm lib\js\owlosui-wire.wasm >nul || exit /b 1
for %%F in (lib\js\owlosui-wire.wasm) do echo lib\js\owlosui-wire.wasm: %%~zF bytes
