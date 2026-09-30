//! OPEN_WINDOW: the server's desktop in a native window.
//!
//! A client that wants a window instead of a console - a C# program built
//! as a Windows application, say - sends OPEN_WINDOW once. From then on the
//! screen is `lib/window` on this process's main thread: it draws the core
//! and feeds it the keys and the mouse, and the client draws nothing.
//! Requests still come on stdin and replies still go to stdout, exactly
//! as before; a thread reads them and wakes the window, and the window's
//! thread runs them, so the core is only ever touched by one thread.
//!
//! What changes for the client is where input comes from. It asks with
//! WAIT, and the reply comes when the window has had a key or a click:
//! what it pressed, what it chose, and the desktop's size. Input that
//! arrives while the client is busy is queued and handed over one event
//! per WAIT, as a console's input buffer would - so a client's loop is
//! the same loop either way, with WAIT where the console read was.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver};

use owlosui_core::{Event, Glyph, Ui};
use owlosui_serve::{op, Server};
use owlosui_window::{run, Options, Program};

/// A request from the reading thread; `None` is the end of stdin.
type Request = Option<(u8, Vec<u8>)>;

struct Hosted<W: Write> {
    server: Server,
    requests: Receiver<Request>,
    out: W,
    /// Input the client has not asked for yet.
    queue: VecDeque<Event>,
    /// A WAIT is outstanding.
    waiting: bool,
    /// An event has gone into the core since that WAIT.
    applied: bool,
    quit: bool,
}

impl<W: Write> Hosted<W> {
    fn reply(&mut self, status: u8, body: &[u8]) {
        let ok = self.out.write_all(&[status]).is_ok()
            && self.out.write_all(&(body.len() as u16).to_le_bytes()).is_ok()
            && self.out.write_all(body).is_ok()
            && self.out.flush().is_ok();
        if !ok {
            // The client has gone: there is nobody left to draw for.
            self.quit = true;
        }
    }

    /// An event for the client's WAIT: into the core now if a WAIT is
    /// outstanding and has had nothing yet, else queued for the next one.
    fn feed(&mut self, ev: Event) {
        if self.waiting && !self.applied {
            self.ui().handle(ev);
            self.applied = true;
        } else {
            self.queue.push_back(ev);
        }
    }
}

impl<W: Write> Program for Hosted<W> {
    fn ui(&mut self) -> &mut Ui {
        self.server.ui_mut().expect("OPEN_WINDOW is refused before INIT")
    }

    fn input(&mut self, ev: Event) {
        match ev {
            // A new size is the window's business, not a question for the
            // client: the core takes it at once, and the next WAIT reply
            // carries it.
            Event::Resize(..) => self.ui().handle(ev),
            _ => self.feed(ev),
        }
    }

    /// The WAIT is answered once its event has been through the core and
    /// any button it pressed has been seen down (the window holds a pick
    /// for a moment, then comes back here).
    fn after_input(&mut self) -> bool {
        if self.waiting && self.applied && !self.ui().pick_pending() {
            let ui = self.ui();
            let pressed = ui.take_pressed().unwrap_or(0);
            let command = ui.take_command().unwrap_or(0);
            let r = ui.rect(ui.root());
            let mut body = Vec::with_capacity(8);
            body.extend_from_slice(&pressed.to_le_bytes());
            body.extend_from_slice(&command.to_le_bytes());
            body.extend_from_slice(&r.w.to_le_bytes());
            body.extend_from_slice(&r.h.to_le_bytes());
            self.waiting = false;
            self.applied = false;
            self.reply(0, &body);
        }
        !self.quit
    }

    fn glyph(&self, g: Glyph) -> char {
        self.server.glyph(g)
    }

    /// Requests from the pipe, run on the window's thread.
    fn woken(&mut self) -> bool {
        while let Ok(request) = self.requests.try_recv() {
            match request {
                None => self.quit = true,
                Some((op::WAIT, _)) => {
                    self.waiting = true;
                    self.applied = false;
                    if let Some(ev) = self.queue.pop_front() {
                        self.feed(ev);
                    }
                }
                Some((op, payload)) => {
                    let (status, body, keep_going) = self.server.call(op, &payload);
                    self.reply(status, &body);
                    if !keep_going {
                        self.quit = true;
                    }
                }
            }
        }
        !self.quit
    }
}

/// Run the window until the client quits or the window is closed. The
/// OK for OPEN_WINDOW has been sent; stdin is read from here on by a
/// thread of its own.
pub fn host(server: Server, title: &str, activate: bool, out: impl Write + 'static) {
    run(&Options { title, activate, ..Options::default() }, move |waker| {
        let (tx, requests) = channel::<Request>();
        std::thread::spawn(move || {
            let mut input = std::io::stdin().lock();
            loop {
                let mut head = [0u8; 3];
                let request = if input.read_exact(&mut head).is_ok() {
                    let mut payload = vec![0u8; u16::from_le_bytes([head[1], head[2]]) as usize];
                    input.read_exact(&mut payload).ok().map(|_| (head[0], payload))
                } else {
                    None
                };
                let end = request.is_none();
                if tx.send(request).is_err() {
                    return;
                }
                waker.wake();
                if end {
                    return;
                }
            }
        });
        Hosted { server, requests, out, queue: VecDeque::new(), waiting: false, applied: false, quit: false }
    });
}
