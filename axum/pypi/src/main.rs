mod cli;

use axum::Router;
use axum::extract::Path;
use axum::response::Redirect;
use axum::routing::get;
use pypi::{AppState, package, packages, project, simple};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let matches = cli::build().get_matches();

    let package_root: PathBuf = matches.get_one::<PathBuf>("PACKAGES").unwrap().clone();

    let host: Ipv4Addr = *matches.get_one::<Ipv4Addr>("HOST").unwrap();

    let port: u16 = *matches.get_one::<u16>("PORT").unwrap();

    let _hash: String = matches.get_one::<String>("HASH").unwrap().clone();

    let debug: bool = matches.get_flag("DEBUG");

    let mut pkg = package::Packages::new(&package_root);
    pkg.collect().unwrap();

    let state = AppState {
        pkg: Arc::new(Mutex::new(pkg)),
        debug,
    };

    let app = Router::new()
        .route("/simple", get(|| async { Redirect::permanent("/simple/") }))
        .route("/simple/", get(simple))
        .route(
            "/simple/{project}",
            get(|Path(project): Path<String>| async move {
                let path = format!("/simple/{project}/");
                Redirect::permanent(&path)
            }),
        )
        .route("/simple/{project}/", get(project))
        .route("/packages/{package}", get(packages))
        .with_state(state);

    let listener = TcpListener::bind((host, port)).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
