mod cli;

use actix_web::{App, HttpServer, web};
use npmjs::{AppState, get_archive, get_package, get_version, package};
use std::io::Result;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[actix_web::main]
async fn main() -> Result<()> {
    let matches = cli::build().get_matches();

    let package_root: PathBuf = matches.get_one::<PathBuf>("PACKAGES").unwrap().clone();

    let host: Ipv4Addr = *matches.get_one::<Ipv4Addr>("HOST").unwrap();

    let port: u16 = *matches.get_one::<u16>("PORT").unwrap();

    let debug: bool = matches.get_flag("DEBUG");

    let mut pkg = package::Packages::new(&package_root);
    pkg.collect().unwrap();

    let state = AppState {
        pkg: Arc::new(Mutex::new(pkg)),
        debug,
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(get_package)
            .service(get_version)
            .service(get_archive)
    })
    .bind((host, port))?
    .run()
    .await
}
