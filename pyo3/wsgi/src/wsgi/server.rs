use super::WsgiService;
use httparse::{EMPTY_HEADER, Request};
use std::io::prelude::*;
use std::io::{ErrorKind, Result};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

fn handle(stream: TcpStream, app: &Path, debug: bool) -> Result<()> {
    let mut stream = stream;

    let remote_addr = stream.peer_addr()?;
    println!("connected from {}", remote_addr);

    let mut header_buffer = vec![];
    loop {
        let mut buf = Vec::new();
        let mut b = [0u8; 1];
        while stream.read_exact(&mut b).is_ok() {
            buf.extend_from_slice(&b);

            if b[0] == b'\n' {
                break;
            }
        }

        if buf.is_empty() {
            break;
        }

        if buf.len() == 2 && buf[0] == b'\r' && buf[1] == b'\n' {
            header_buffer.extend_from_slice(&buf);
            break;
        }

        header_buffer.extend_from_slice(&buf);
    }

    let mut headers = [EMPTY_HEADER; 64];
    let mut req = Request::new(&mut headers);
    let status = req.parse(&header_buffer).unwrap();

    if status.is_partial() {
        println!("{:?}: {:?}", status, req);
        stream.write_all(b"HTTP/1.0 500 INTERNAL SERVER ERROR\r\n")?;
        stream.write_all(b"\r\n")?;
    } else {
        let service = WsgiService::new(app, remote_addr, debug);
        if let Err(e) = service.exec(req, &mut stream) {
            println!("{:?}", e);
            stream.write_all(b"HTTP/1.1 500 INTERNAL SERVER ERROR\r\n")?;
            stream.write_all(b"\r\n")?;
        }
    }

    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

pub struct WsgiServer {
    application: PathBuf,
    debug: bool,
}

impl WsgiServer {
    pub fn new(application: &Path, debug: bool) -> WsgiServer {
        WsgiServer {
            application: application.to_path_buf(),
            debug,
        }
    }

    pub fn serve_forever(&self, addr: SocketAddr, running: Arc<AtomicBool>) {
        let listener = TcpListener::bind(addr).unwrap();
        listener.set_nonblocking(true).unwrap();

        println!("listening on {}", addr);

        while running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _peer_addr)) => {
                    let app = self.application.clone();
                    let debug = self.debug;
                    thread::spawn(move || {
                        if let Err(e) = handle(stream, &app, debug) {
                            println!("connection handler error: {:?}", e);
                        }
                    });
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("accept error: {}", e);
                    break;
                }
            }
        }

        println!("server stopped");
    }
}
