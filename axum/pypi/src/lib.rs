pub mod error;
pub mod model;
pub mod package;

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use chrono::format::SecondsFormat;
use chrono::{DateTime, Utc};
use md5::{Digest, Md5};
use regex::Regex;
use sha2::Sha256;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path;
use std::sync::{Arc, LazyLock};
use tokio::fs::File;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

const JSON_TYPE: &str = "application/vnd.pypi.simple.v1+json";

static PROJECT_NORMALIZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[-_.]+").unwrap());

#[derive(Clone)]
pub struct AppState {
    pub pkg: Arc<Mutex<package::Packages>>,
    pub hash: String,
    pub debug: bool,
}

pub async fn simple(State(state): State<AppState>) -> (HeaderMap, Json<model::SimpleResponse>) {
    let response = model::SimpleResponse {
        meta: model::SimpleMetadata {
            api_version: "1.4".to_owned(),
        },
        projects: projects(&state).await,
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

    let (files, versoins) = files(&state, &project, host).await;

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
        let pkg = state.pkg.lock().await;
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

async fn projects(state: &AppState) -> Vec<model::SimpleProject> {
    let pkg = state.pkg.lock().await;

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

async fn files(
    state: &AppState,
    project: &str,
    host: &str,
) -> (Vec<model::ProjectFile>, HashSet<String>) {
    let mut pkg = state.pkg.lock().await;
    let hash = &state.hash;

    let mut pkgs = vec![];
    let mut versions = HashSet::new();
    for file in pkg.files.iter_mut() {
        let name = normalize(&file.distribution);
        let version = file.version.clone();
        if name == project {
            pkgs.push(file);
            versions.insert(version);
        }
    }

    let mut files = vec![];
    for pkg in pkgs.iter_mut() {
        let hash_value = if !pkg.hashes.contains_key(hash) {
            insert_hash(hash, pkg).await
        } else {
            pkg.hashes.get(hash).unwrap().clone()
        };
        let mut hashes = HashMap::new();
        hashes.insert(hash.clone(), hash_value);

        let update_time: Option<DateTime<Utc>> = pkg.updated_at.map(|t| t.into());

        let file = model::ProjectFile {
            filename: pkg.filename.clone(),
            url: format!("http://{}/packages/{}", host, &pkg.filename),
            hashes,
            size: pkg.size,
            upload_time: update_time.map(|t| t.to_rfc3339_opts(SecondsFormat::Micros, true)),
            ..Default::default()
        };
        files.push(file);
    }

    (files, versions)
}

async fn insert_hash(algo: &str, pkg: &mut &mut package::Package) -> String {
    let hash = match algo {
        "md5" => compute_hash::<Md5>(&pkg.path).await.unwrap(),
        "sha256" => compute_hash::<Sha256>(&pkg.path).await.unwrap(),
        _ => panic!("unknown hash algorithm"),
    };
    pkg.hashes.insert(algo.to_owned(), hash.clone());
    hash
}

async fn compute_hash<D: Default + Digest>(path: &path::Path) -> Result<String, error::Error> {
    let file = File::open(path).await?;
    let mut stream = ReaderStream::new(file);

    let mut hasher = D::default();
    while let Some(chunk) = stream.next().await {
        hasher.update(chunk?);
    }

    let hash = hasher.finalize();
    Ok(hash.iter().map(|v| format!("{:02x}", v)).collect())
}

fn normalize(name: &str) -> String {
    PROJECT_NORMALIZE.replace_all(name, "-").to_string()
}
