mod cli;

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wsgi::wsgi::WsgiServer;

fn main() {
    let matches = cli::build().get_matches();

    let application: PathBuf = matches.get_one::<PathBuf>("APPLICATION").unwrap().clone();

    let host: Ipv4Addr = *matches.get_one::<Ipv4Addr>("HOST").unwrap();

    let port: u16 = *matches.get_one::<u16>("PORT").unwrap();

    let debug: bool = matches.get_flag("DEBUG");

    println!("serving ... {:?}:{:?}", &host, port);
    println!("application : {:?}", &application);
    println!("debug : {:?}", &debug);

    let running = Arc::new(AtomicBool::new(true));
    let shutdown_flag = Arc::clone(&running);

    ctrlc::set_handler(move || {
        if shutdown_flag.swap(false, Ordering::SeqCst) {
            println!("\nSIGINT received, shutting down...");
        }
    })
    .unwrap();

    let server = WsgiServer::new(&application, debug);

    server.serve_forever((host, port).into(), running);
}
