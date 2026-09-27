//! The Win32 this program needs, declared by hand: a window, its
//! messages, and enough GDI to fill rectangles and write characters.
//! Signatures follow the SDK's headers for 64-bit Windows.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use core::ffi::c_void;

pub type HANDLE = *mut c_void;
pub type HWND = HANDLE;
pub type HDC = HANDLE;
pub type HGDIOBJ = HANDLE;
pub type HINSTANCE = HANDLE;
pub type HICON = HANDLE;
pub type HCURSOR = HANDLE;
pub type HBRUSH = HANDLE;
pub type HMENU = HANDLE;
pub type HFONT = HANDLE;
pub type HBITMAP = HANDLE;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type UINT = u32;
pub type DWORD = u32;
pub type BOOL = i32;
pub type LONG = i32;
pub type ATOM = u16;
pub type COLORREF = u32;
pub type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RECT {
    pub left: LONG,
    pub top: LONG,
    pub right: LONG,
    pub bottom: LONG,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SIZE {
    pub cx: LONG,
    pub cy: LONG,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

#[repr(C)]
pub struct WNDCLASSW {
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [u8; 32],
}

pub const CS_DBLCLKS: UINT = 0x0008;
pub const CS_HREDRAW: UINT = 0x0002;
pub const CS_VREDRAW: UINT = 0x0001;
pub const WS_OVERLAPPEDWINDOW: DWORD = 0x00CF0000;
pub const WS_VISIBLE: DWORD = 0x10000000;
pub const CW_USEDEFAULT: i32 = 0x80000000u32 as i32;
pub const SW_SHOW: i32 = 5;
pub const IDC_ARROW: usize = 32512;

pub const WM_DESTROY: UINT = 0x0002;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_CHAR: UINT = 0x0102;
pub const WM_SYSKEYDOWN: UINT = 0x0104;
pub const WM_SYSCHAR: UINT = 0x0106;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
pub const WM_RBUTTONDOWN: UINT = 0x0204;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_RBUTTONDBLCLK: UINT = 0x0206;
pub const WM_MBUTTONDOWN: UINT = 0x0207;
pub const WM_MBUTTONUP: UINT = 0x0208;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_GETMINMAXINFO: UINT = 0x0024;

pub const VK_BACK: usize = 0x08;
pub const VK_TAB: usize = 0x09;
pub const VK_RETURN: usize = 0x0D;
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_ESCAPE: usize = 0x1B;
pub const VK_PRIOR: usize = 0x21;
pub const VK_NEXT: usize = 0x22;
pub const VK_END: usize = 0x23;
pub const VK_HOME: usize = 0x24;
pub const VK_LEFT: usize = 0x25;
pub const VK_UP: usize = 0x26;
pub const VK_RIGHT: usize = 0x27;
pub const VK_DOWN: usize = 0x28;
pub const VK_INSERT: usize = 0x2D;
pub const VK_DELETE: usize = 0x2E;
pub const VK_F1: usize = 0x70;
pub const VK_F12: usize = 0x7B;

pub const FW_NORMAL: i32 = 400;
pub const DEFAULT_CHARSET: DWORD = 1;
pub const OUT_TT_PRECIS: DWORD = 4;
pub const CLIP_DEFAULT_PRECIS: DWORD = 0;
pub const CLEARTYPE_QUALITY: DWORD = 5;
pub const FIXED_PITCH: DWORD = 1;
pub const OPAQUE: i32 = 2;
pub const ETO_OPAQUE: UINT = 0x0002;
pub const SRCCOPY: DWORD = 0x00CC0020;

#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassW(cls: *const WNDCLASSW) -> ATOM;
    pub fn CreateWindowExW(
        ex: DWORD,
        class: *const u16,
        title: *const u16,
        style: DWORD,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: HMENU,
        inst: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(h: HWND, m: UINT, w: WPARAM, l: LPARAM) -> LRESULT;
    pub fn GetMessageW(msg: *mut MSG, h: HWND, min: UINT, max: UINT) -> BOOL;
    pub fn TranslateMessage(msg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(code: i32);
    pub fn DestroyWindow(h: HWND) -> BOOL;
    pub fn ShowWindow(h: HWND, cmd: i32) -> BOOL;
    pub fn UpdateWindow(h: HWND) -> BOOL;
    pub fn InvalidateRect(h: HWND, r: *const RECT, erase: BOOL) -> BOOL;
    pub fn BeginPaint(h: HWND, ps: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(h: HWND, ps: *const PAINTSTRUCT) -> BOOL;
    pub fn GetClientRect(h: HWND, r: *mut RECT) -> BOOL;
    pub fn GetDC(h: HWND) -> HDC;
    pub fn ReleaseDC(h: HWND, dc: HDC) -> i32;
    pub fn LoadCursorW(inst: HINSTANCE, name: *const u16) -> HCURSOR;
    pub fn SetTimer(h: HWND, id: usize, ms: UINT, f: *const c_void) -> usize;
    pub fn KillTimer(h: HWND, id: usize) -> BOOL;
    pub fn GetKeyState(vk: i32) -> i16;
    pub fn SetCapture(h: HWND) -> HWND;
    pub fn ReleaseCapture() -> BOOL;
    pub fn SetWindowTextW(h: HWND, s: *const u16) -> BOOL;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateFontW(
        h: i32,
        w: i32,
        esc: i32,
        orient: i32,
        weight: i32,
        italic: DWORD,
        underline: DWORD,
        strike: DWORD,
        charset: DWORD,
        out_prec: DWORD,
        clip_prec: DWORD,
        quality: DWORD,
        pitch: DWORD,
        face: *const u16,
    ) -> HFONT;
    pub fn SelectObject(dc: HDC, o: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(o: HGDIOBJ) -> BOOL;
    pub fn SetTextColor(dc: HDC, c: COLORREF) -> COLORREF;
    pub fn SetBkColor(dc: HDC, c: COLORREF) -> COLORREF;
    pub fn SetBkMode(dc: HDC, mode: i32) -> i32;
    pub fn ExtTextOutW(dc: HDC, x: i32, y: i32, opts: UINT, r: *const RECT, s: *const u16, n: UINT, dx: *const i32) -> BOOL;
    pub fn GetTextExtentPoint32W(dc: HDC, s: *const u16, n: i32, size: *mut SIZE) -> BOOL;
    pub fn CreateSolidBrush(c: COLORREF) -> HBRUSH;
    pub fn CreateCompatibleDC(dc: HDC) -> HDC;
    pub fn CreateCompatibleBitmap(dc: HDC, w: i32, h: i32) -> HBITMAP;
    pub fn DeleteDC(dc: HDC) -> BOOL;
    pub fn BitBlt(dst: HDC, x: i32, y: i32, w: i32, h: i32, src: HDC, sx: i32, sy: i32, rop: DWORD) -> BOOL;
    pub fn CreateBitmap(w: i32, h: i32, planes: UINT, bits: UINT, data: *const c_void) -> HBITMAP;
    pub fn CreatePatternBrush(b: HBITMAP) -> HBRUSH;
}

#[link(name = "user32")]
extern "system" {
    pub fn FillRect(dc: HDC, r: *const RECT, b: HBRUSH) -> i32;
}

/// A NUL-terminated UTF-16 string for the API.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}

pub fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}
