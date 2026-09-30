//! The wire server inside a WebAssembly module.
//!
//! Three functions, and they are the whole transport:
//!
//! * `owl_in(len)` - room for a request's payload; the page writes the bytes
//!   there, into this module's memory;
//! * `owl_call(op)` - runs the request, returns the reply's length;
//! * `owl_out()` - where the reply is: `status:u8 len:u16 body`, exactly what
//!   the pipe carries for the C# client.
//!
//! So `lib/js/owlosui.js` speaks lib/PROTOCOL.md byte for byte, the same as
//! `Owlosui.cs` does over a pipe - it only has no pipe. One module, one
//! screen: the server lives in a static, and WebAssembly runs it on one
//! thread, in call order.

use owlosui_serve::Server;

static mut SERVER: Option<Server> = None;
static mut IN: Vec<u8> = Vec::new();
static mut OUT: Vec<u8> = Vec::new();

#[allow(static_mut_refs)]
fn server() -> &'static mut Server {
    unsafe { SERVER.get_or_insert_with(Server::new) }
}

/// Room for `len` bytes of payload; the caller writes them at the pointer.
#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn owl_in(len: u32) -> *mut u8 {
    unsafe {
        IN.clear();
        IN.resize(len as usize, 0);
        IN.as_mut_ptr()
    }
}

/// Run the request `op` with the payload in the `owl_in` buffer. Returns the
/// length of the reply at `owl_out`: three bytes of head and the body.
#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn owl_call(op: u8) -> u32 {
    unsafe {
        let (status, body, _going) = server().call(op, &IN);
        OUT.clear();
        OUT.push(status);
        OUT.extend_from_slice(&(body.len() as u16).to_le_bytes());
        OUT.extend_from_slice(&body);
        OUT.len() as u32
    }
}

/// Where the last reply is.
#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn owl_out() -> *const u8 {
    unsafe { OUT.as_ptr() }
}
