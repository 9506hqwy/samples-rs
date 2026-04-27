pub mod error;
pub mod model;
pub mod package;

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

const JSON_TYPE: &str = "application/vnd.pypi.simple.v1+json";

static PROJECT_NORMALIZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[-_.]+").unwrap());

#[derive(Clone)]
pub struct AppState {
    pub pkg: Arc<Mutex<package::Packages>>,
    pub debug: bool,
}

pub async fn simple(State(state): State<AppState>) -> (HeaderMap, Json<model::SimpleResponse>) {
    let response = model::SimpleResponse {
        meta: model::SimpleMetadata {
            api_version: "1.4".to_owned(),
        },
        projects: projects(&state),
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, JSON_TYPE.parse().unwrap());

    (headers, Json(response))
}

pub async fn project(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project): Path<String>,
) -> (HeaderMap, Json<model::ProjectResponse>) {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");

    let (files, versoins) = files(&state, &project, host);

    let response = model::ProjectResponse {
        meta: model::ProjectMetadata {
            api_version: "1.4".to_owned(),
        },
        files,
        name: project,
        versions: Some(versoins.into_iter().collect()),
        ..Default::default()
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, JSON_TYPE.parse().unwrap());

    (headers, Json(response))
}

pub async fn packages(
    State(state): State<AppState>,
    Path(package): Path<String>,
) -> impl IntoResponse {
    let path = {
        let pkg = state.pkg.lock().unwrap();
        let file = pkg.files.iter().find(|f| f.filename == package);
        file.map(|f| f.path.clone())
    };
    if path.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let fd = File::open(&path.unwrap()).await.unwrap();
    let stream = ReaderStream::new(fd);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());

    Ok((headers, body))
}

fn projects(state: &AppState) -> Vec<model::SimpleProject> {
    let pkg = state.pkg.lock().unwrap();

    let mut names = HashSet::new();
    for file in &pkg.files {
        let name = normalize(&file.distribution);
        names.insert(name);
    }

    names
        .into_iter()
        .map(|n| model::SimpleProject { name: n })
        .collect()
}

fn files(
    state: &AppState,
    project: &str,
    host: &str,
) -> (Vec<model::ProjectFile>, HashSet<String>) {
    let pkg = state.pkg.lock().unwrap();

    let mut pkgs = vec![];
    let mut versions = HashSet::new();
    for file in &pkg.files {
        let name = normalize(&file.distribution);
        if name == project {
            pkgs.push(file);
            versions.insert(file.version.clone());
        }
    }

    let mut files = vec![];
    for pkg in &pkgs {
        let file = model::ProjectFile {
            filename: pkg.filename.clone(),
            url: format!("http://{}/packages/{}", host, &pkg.filename),
            size: pkg.size,
            ..Default::default()
        };
        files.push(file);
    }

    (files, versions)
}

fn normalize(name: &str) -> String {
    PROJECT_NORMALIZE.replace_all(name, "-").to_string()
}
