pub mod error;
pub mod model;
pub mod package;

use actix_files::NamedFile;
use actix_web::{Either, HttpRequest, HttpResponse, Responder, Result, get, http, web};
use chrono::format::SecondsFormat;
use chrono::{DateTime, Utc};
use percent_encoding::{AsciiSet, CONTROLS, percent_encode};
use sha1::{Digest, Sha1};
use std::path::Path;
use std::{collections::HashMap, sync::Arc};
use tokio::fs::File;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

const JSON_TYPE: &str = "application/vnd.npm.install-v1+json";

// https://url.spec.whatwg.org/#path-percent-encode-set
const QUERY_SET: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');
const PATH_SET: &AsciiSet = &QUERY_SET.add(b'?').add(b'^').add(b'`').add(b'{').add(b'}');
const PATH_SEGMENT_SET: &AsciiSet = &PATH_SET.add(b'/');

#[derive(Clone)]
pub struct AppState {
    pub pkg: Arc<Mutex<package::Packages>>,
    pub debug: bool,
}

#[get("/{package}")]
async fn get_package(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let host = get_header_value(req.headers(), http::header::HOST.as_str(), "localhost");

    let package = path.into_inner();

    if let Some(pkg) = packages(data, &package, &host).await {
        Ok(HttpResponse::Ok()
            .insert_header(http::header::ContentType(JSON_TYPE.parse().unwrap()))
            .json(pkg))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[get("/{package}/{version}")]
async fn get_version(
    data: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let host = get_header_value(req.headers(), http::header::HOST.as_str(), "localhost");

    let (package, version) = path.into_inner();

    let mut pkg = packages(data, &package, &host).await;

    if let Some(version) = pkg.as_mut().and_then(|p| p.versions.remove(&version)) {
        Ok(HttpResponse::Ok()
            .insert_header(http::header::ContentType(JSON_TYPE.parse().unwrap()))
            .json(version))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[get("/{package}/-/{filename}")]
async fn get_archive(
    data: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (_, filename) = path.into_inner();

    let pkg = data.pkg.lock().await;

    let pkgs: Vec<&package::Package> = pkg
        .files
        .iter()
        .filter(|f| f.filename == filename)
        .collect();

    if let Some(pkg) = pkgs.first()
        && let Ok(file) = NamedFile::open_async(&pkg.path).await
    {
        Either::Left(file)
    } else {
        Either::Right(HttpResponse::NotFound().finish())
    }
}

async fn packages(data: web::Data<AppState>, name: &str, host: &str) -> Option<model::Package> {
    let mut pkg = data.pkg.lock().await;

    let mut pkgs: Vec<&mut package::Package> = pkg
        .files
        .iter_mut()
        .filter(|f| f.metadata.name == name)
        .collect();
    pkgs.sort();
    if pkgs.is_empty() {
        return None;
    }

    let versions = model_versions(pkgs.as_mut(), host).await;
    let time = model_time(&pkgs);

    let latest = pkgs.last().unwrap();
    let author = model_author(&latest.metadata.author);
    let repository = model_repository(&latest.metadata.repository);

    Some(model::Package {
        name: latest.metadata.name.clone(),
        versions,
        time,
        author,
        description: latest.metadata.description.clone(),
        repository,
        // TODO:
        ..Default::default()
    })
}

async fn model_versions(
    pkgs: &mut [&mut package::Package],
    host: &str,
) -> HashMap<String, model::PackageVersion> {
    let mut versions = HashMap::new();

    for pkg in pkgs {
        let m = model_version(pkg, host).await;
        versions.insert(m.version.clone(), m);
    }

    versions
}

async fn model_version(pkg: &mut package::Package, host: &str) -> model::PackageVersion {
    if pkg.shasum.is_empty() {
        pkg.shasum = compute_hash::<Sha1>(&pkg.path).await.unwrap();
    }

    let name = pkg.metadata.name.clone();
    let encoded_name = percent_encode(name.as_bytes(), PATH_SEGMENT_SET).to_string();
    let filename = pkg.filename.clone();

    model::PackageVersion {
        name: name.clone(),
        version: pkg.metadata.version.clone(),
        dependencies: pkg.metadata.dependencies.clone(),
        dev_dependencies: pkg.metadata.dev_dependencies.clone(),
        dist: model::PackageDist {
            shasum: pkg.shasum.clone(),
            tarball: format!("http://{host}/{encoded_name}/-/{filename}"),
        },
    }
}

fn model_author(author: &Option<package::PackageJsonPeople>) -> Option<model::PackageAuthor> {
    author.as_ref().map(|author| match author {
        package::PackageJsonPeople::Line(author) => model::PackageAuthor {
            name: author.clone(),
            ..Default::default()
        },
        package::PackageJsonPeople::Object(author) => model::PackageAuthor {
            name: author.name.clone(),
            email: author.email.clone(),
            url: author.url.clone(),
        },
    })
}

fn model_repository(
    repository: &Option<package::PackageJsonRepository>,
) -> Option<model::PackageRepository> {
    repository.as_ref().map(|repo| match repo {
        package::PackageJsonRepository::Line(repo) => model::PackageRepository {
            url: repo.clone(),
            ..Default::default()
        },
        package::PackageJsonRepository::Object(repo) => model::PackageRepository {
            ty: repo.ty.clone(),
            url: repo.url.clone(),
        },
    })
}

fn model_time(pkgs: &[&mut package::Package]) -> HashMap<String, String> {
    let mut times = HashMap::new();

    let oldest = pkgs.first().unwrap();
    let oldest_time: DateTime<Utc> = oldest.created_at.map(|t| t.into()).unwrap();
    times.insert(
        "created".to_string(),
        oldest_time.to_rfc3339_opts(SecondsFormat::Micros, true),
    );

    let latest = pkgs.last().unwrap();
    let latest_time: DateTime<Utc> = latest.created_at.map(|t| t.into()).unwrap();
    times.insert(
        "modified".to_string(),
        latest_time.to_rfc3339_opts(SecondsFormat::Micros, true),
    );

    for pkg in pkgs {
        let time: DateTime<Utc> = pkg.created_at.map(|t| t.into()).unwrap();
        times.insert(
            pkg.metadata.version.clone(),
            time.to_rfc3339_opts(SecondsFormat::Micros, true),
        );
    }

    times
}

async fn compute_hash<D: Default + Digest>(path: &Path) -> Result<String, error::Error> {
    let file = File::open(path).await?;
    let mut stream = ReaderStream::new(file);

    let mut hasher = D::default();
    while let Some(chunk) = stream.next().await {
        hasher.update(chunk?);
    }

    let hash = hasher.finalize();
    Ok(hash.iter().map(|v| format!("{:02x}", v)).collect())
}

fn get_header_value(headers: &http::header::HeaderMap, name: &str, default: &str) -> String {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or(default)
        .to_string()
}
