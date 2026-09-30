//! The core behind a pipe: `Server` (lib.rs) on stdin and stdout.
//!
//! A client starts this program, writes requests to its stdin and reads one
//! reply per request from its stdout, in the format of `lib/PROTOCOL.md`.
//! Everything that means anything is in lib.rs; this is the plumbing - and
//! OPEN_WINDOW, the one request only this host can answer, because a
//! window needs a process of its own to live in (window.rs).

use std::io::{Read, Write};

use owlosui_serve::{op, Server};

#[cfg(windows)]
mod window;

fn main() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut output = std::io::BufWriter::new(stdout.lock());
    let mut server = Server::new();

    loop {
        // op:u8 len:u16, then the payload. End of input is a normal way for a
        // client to leave: it closed the pipe, and so do we.
        let mut head = [0u8; 3];
        let mut input = std::io::stdin().lock();
        if input.read_exact(&mut head).is_err() {
            return Ok(());
        }
        let op = head[0];
        let len = u16::from_le_bytes([head[1], head[2]]) as usize;
        let mut payload = vec![0u8; len];
        input.read_exact(&mut payload)?;
        drop(input);

        let (status, body, keep_going) = match op {
            op::OPEN_WINDOW => match open_window(&mut server, &payload) {
                Ok((title, activate)) => {
                    reply(&mut output, 0, &[])?;
                    #[cfg(windows)]
                    window::host(server, &title, activate, output);
                    #[cfg(not(windows))]
                    let _ = (title, activate);
                    return Ok(());
                }
                Err(e) => {
                    let mut msg = (e.len() as u16).to_le_bytes().to_vec();
                    msg.extend_from_slice(e.as_bytes());
                    (1, msg, true)
                }
            },
            _ => server.call(op, &payload),
        };
        reply(&mut output, status, &body)?;

        if !keep_going {
            return Ok(());
        }
    }
}

fn reply(output: &mut impl Write, status: u8, body: &[u8]) -> std::io::Result<()> {
    output.write_all(&[status])?;
    output.write_all(&(body.len() as u16).to_le_bytes())?;
    output.write_all(body)?;
    output.flush()
}

/// Whether OPEN_WINDOW can be honoured; if so the window's title, and
/// whether it takes the focus (flags bit 0 clear) - `title:str [flags:u8]`.
fn open_window(server: &mut Server, payload: &[u8]) -> Result<(String, bool), String> {
    if !cfg!(windows) {
        return Err("a window needs Windows; this server has only the pipe".into());
    }
    if server.ui_mut().is_none() {
        return Err("INIT first".into());
    }
    let n = payload.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize).ok_or("title missing")?;
    let title = payload.get(2..2 + n).ok_or("title cut short")?;
    let title = String::from_utf8(title.to_vec()).map_err(|_| "title is not UTF-8")?;
    let flags = payload.get(2 + n).copied().unwrap_or(0);
    Ok((title, flags & 1 == 0))
}
