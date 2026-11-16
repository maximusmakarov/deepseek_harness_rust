//! End-to-end resolver tests against a local registry stub (no network).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use base64::Engine;
use dshr_npm::{InstallOptions, Installer, PackageSpec};

/// Minimal HTTP server: exact-path routes served from a shared map.
struct Stub {
    base: String,
    routes: Arc<std::sync::Mutex<HashMap<String, Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Stub {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.set_nonblocking(true).expect("nonblocking");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        let routes = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let shared = Arc::clone(&routes);
        let handle = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    // One thread per connection: a slow client must not wedge
                    // the accept loop (tests run in parallel).
                    Ok((stream, _)) => {
                        let routes = Arc::clone(&shared);
                        std::thread::spawn(move || respond(stream, &routes));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            base,
            routes,
            stop,
            handle: Some(handle),
        }
    }

    fn insert(&self, path: String, body: Vec<u8>) {
        let body = match String::from_utf8(body) {
            Ok(text) => text.replace("REPLACED", &self.base).into_bytes(),
            Err(error) => error.into_bytes(),
        };
        self.routes.lock().expect("routes").insert(path, body);
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn respond(mut stream: TcpStream, routes: &std::sync::Mutex<HashMap<String, Vec<u8>>>) {
    let mut buffer = [0u8; 2048];
    let read = stream.read(&mut buffer).unwrap_or(0);
    let request = String::from_utf8_lossy(&buffer[..read]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    let body = routes.lock().expect("routes").get(path).cloned();
    let (status, body) = match body {
        Some(body) => ("200 OK", body),
        None => ("404 Not Found", b"{\"error\":\"not found\"}".to_vec()),
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(&body);
}

fn sri(bytes: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA512, bytes);
    format!(
        "sha512-{}",
        base64::engine::general_purpose::STANDARD.encode(digest.as_ref())
    )
}

/// Builds a `.tar.gz` npm tarball with `package/package.json` and optional files.
fn append_file<W: Write>(builder: &mut tar::Builder<W>, path: &str, body: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(body.len() as u64);
    header.set_cksum();
    builder
        .append_data(&mut header, path, body)
        .expect("append");
}

fn tarball(package_json: &str, files: &[(&str, &str)]) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    append_file(
        &mut builder,
        "package/package.json",
        package_json.as_bytes(),
    );
    for (path, body) in files {
        append_file(&mut builder, &format!("package/{path}"), body.as_bytes());
    }
    builder.into_inner().expect("tar").finish().expect("gzip")
}

struct Package {
    name: &'static str,
    version: &'static str,
    dependencies: &'static [(&'static str, &'static str)],
}

/// Builds one packument (all versions of `name`) plus its tarball routes.
fn build_packument(
    name: &str,
    versions: &[(&'static str, &'static [(&'static str, &'static str)])],
) -> (Vec<u8>, Vec<(String, Vec<u8>)>) {
    let mut tarballs = Vec::new();
    let mut encoded_versions = serde_json::Map::new();
    let mut latest: Option<semver::Version> = None;
    for (version, dependencies) in versions {
        let tarball_path = format!("/tarballs/{name}-{version}.tgz");
        let body = tarball(
            &format!("{{\"name\":\"{name}\",\"version\":\"{version}\"}}"),
            &[("index.js", "module.exports = 1;\n")],
        );
        let integrity = sri(&body);
        tarballs.push((tarball_path.clone(), body));
        let dependencies: serde_json::Map<String, serde_json::Value> = dependencies
            .iter()
            .map(|(dep, requirement)| {
                (
                    (*dep).to_owned(),
                    serde_json::Value::String((*requirement).to_owned()),
                )
            })
            .collect();
        encoded_versions.insert(
            (*version).to_owned(),
            serde_json::json!({
                "version": version,
                "dependencies": dependencies,
                "dist": { "tarball": format!("REPLACED{tarball_path}"), "integrity": integrity },
            }),
        );
        let parsed = semver::Version::parse(version).expect("version");
        if latest.as_ref().is_none_or(|current| parsed > *current) {
            latest = Some(parsed);
        }
    }
    let latest = latest.expect("at least one version").to_string();
    let packument = serde_json::json!({
        "name": name,
        "dist-tags": { "latest": latest },
        "versions": encoded_versions,
    });
    (serde_json::to_vec(&packument).expect("packument"), tarballs)
}

/// One published version: version string plus its dependency list.
type VersionSpec = (&'static str, &'static [(&'static str, &'static str)]);

/// Versions grouped by package name.
type GroupedVersions = std::collections::BTreeMap<&'static str, Vec<VersionSpec>>;

/// Registers every package, grouping versions by name.
fn registry(routes: &mut HashMap<String, Vec<u8>>, packages: &[Package]) {
    let mut grouped: GroupedVersions = std::collections::BTreeMap::new();
    for package in packages {
        grouped
            .entry(package.name)
            .or_default()
            .push((package.version, package.dependencies));
    }
    for (name, versions) in grouped {
        let (body, tarballs) = build_packument(name, &versions);
        for (path, tarball) in tarballs {
            routes.insert(path, tarball);
        }
        routes.insert(format!("/{name}"), body);
    }
}

/// Starts a stub and registers the routes on it (tarball URLs are patched to
/// the live address).
fn start(routes: HashMap<String, Vec<u8>>) -> Stub {
    let stub = Stub::start();
    for (path, body) in routes {
        stub.insert(path, body);
    }
    stub
}

#[tokio::test]
async fn installs_transitive_dependencies() {
    let mut routes = HashMap::new();
    registry(
        &mut routes,
        &[
            Package {
                name: "a",
                version: "1.1.0",
                dependencies: &[("b", "^2.0.0")],
            },
            Package {
                name: "b",
                version: "2.3.0",
                dependencies: &[],
            },
        ],
    );
    let stub = start(routes);
    let home = tempfile::tempdir().expect("home");
    let target = tempfile::tempdir().expect("target");
    let installer = Installer::with_registry(&stub.base, home.path());

    let report = installer
        .install(
            target.path(),
            &[PackageSpec::parse("a@^1").expect("spec")],
            &InstallOptions::default(),
        )
        .await
        .expect("install");

    assert_eq!(report.version_of("a"), Some("1.1.0"));
    assert_eq!(report.version_of("b"), Some("2.3.0"));
    assert_eq!(report.path_of("b"), Some("node_modules/b"));
    assert!(target.path().join("node_modules/a/index.js").is_file());
    assert!(target.path().join("node_modules/b/index.js").is_file());
    let lock = std::fs::read_to_string(&report.lock_path).expect("lock");
    assert!(lock.contains("a@1.1.0"), "{lock}");
    assert!(lock.contains("b@2.3.0"), "{lock}");
    assert!(lock.contains("\"roots\""), "{lock}");
}

#[tokio::test]
async fn nests_conflicting_versions() {
    let mut routes = HashMap::new();
    registry(
        &mut routes,
        &[
            Package {
                name: "a",
                version: "1.0.0",
                dependencies: &[("b", "^1.0.0")],
            },
            Package {
                name: "c",
                version: "1.0.0",
                dependencies: &[("b", "^2.0.0")],
            },
            Package {
                name: "b",
                version: "1.5.0",
                dependencies: &[],
            },
            Package {
                name: "b",
                version: "2.5.0",
                dependencies: &[],
            },
        ],
    );
    let stub = start(routes);
    let home = tempfile::tempdir().expect("home");
    let target = tempfile::tempdir().expect("target");
    let installer = Installer::with_registry(&stub.base, home.path());

    let report = installer
        .install(
            target.path(),
            &[
                PackageSpec::parse("a").expect("a"),
                PackageSpec::parse("c").expect("c"),
            ],
            &InstallOptions::default(),
        )
        .await
        .expect("install");

    assert_eq!(
        report.version_of("b"),
        Some("1.5.0"),
        "first version hoists"
    );
    assert!(
        target
            .path()
            .join("node_modules/c/node_modules/b/package.json")
            .is_file(),
        "conflicting version nests under its dependent"
    );
    let nested = std::fs::read_to_string(
        target
            .path()
            .join("node_modules/c/node_modules/b/package.json"),
    )
    .expect("nested package.json");
    assert!(nested.contains("2.5.0"), "{nested}");
}

#[tokio::test]
async fn reuses_one_hoisted_copy_for_compatible_dependents() {
    let mut routes = HashMap::new();
    registry(
        &mut routes,
        &[
            Package {
                name: "a",
                version: "1.0.0",
                dependencies: &[("b", "^2.0.0")],
            },
            Package {
                name: "c",
                version: "1.0.0",
                dependencies: &[("b", "^2.0.0")],
            },
            Package {
                name: "b",
                version: "2.1.0",
                dependencies: &[],
            },
        ],
    );
    let stub = start(routes);
    let home = tempfile::tempdir().expect("home");
    let target = tempfile::tempdir().expect("target");
    let installer = Installer::with_registry(&stub.base, home.path());

    let report = installer
        .install(
            target.path(),
            &[
                PackageSpec::parse("a").expect("a"),
                PackageSpec::parse("c").expect("c"),
            ],
            &InstallOptions::default(),
        )
        .await
        .expect("install");

    assert_eq!(report.packages.len(), 3, "b is installed once");
    assert!(!target.path().join("node_modules/c/node_modules").exists());
    let cas = std::fs::read_dir(home.path().join("npm/cas"))
        .expect("cas")
        .count();
    assert_eq!(cas, 3, "one content-addressed copy per package version");
}

#[tokio::test]
async fn rejects_a_tarball_with_a_wrong_integrity() {
    let (body, tarballs) = build_packument("a", &[("1.0.0", &[])]);
    let mut packument: serde_json::Value = serde_json::from_slice(&body).expect("json");
    packument["versions"]["1.0.0"]["dist"]["integrity"] = serde_json::json!("sha512-AAAA");
    let mut routes = HashMap::new();
    for (path, tarball) in tarballs {
        routes.insert(path, tarball);
    }
    routes.insert(
        "/a".to_owned(),
        serde_json::to_vec(&packument).expect("json"),
    );
    let stub = start(routes);
    let home = tempfile::tempdir().expect("home");
    let target = tempfile::tempdir().expect("target");
    let installer = Installer::with_registry(&stub.base, home.path());

    let error = installer
        .install(
            target.path(),
            &[PackageSpec::parse("a").expect("spec")],
            &InstallOptions::default(),
        )
        .await
        .expect_err("must fail");
    assert!(
        error.to_string().contains("integrity check failed"),
        "unexpected error: {error}"
    );
    assert!(!target.path().join("node_modules").exists());
    let cas = home.path().join("npm/cas");
    assert!(!cas.exists() || std::fs::read_dir(&cas).expect("cas").count() == 0);
}

#[tokio::test]
async fn offline_install_reuses_cached_metadata_and_store() {
    let mut routes = HashMap::new();
    registry(
        &mut routes,
        &[Package {
            name: "a",
            version: "1.0.0",
            dependencies: &[],
        }],
    );
    let stub = start(routes);
    let home = tempfile::tempdir().expect("home");
    let target = tempfile::tempdir().expect("target");
    let spec = [PackageSpec::parse("a").expect("spec")];
    Installer::with_registry(&stub.base, home.path())
        .install(target.path(), &spec, &InstallOptions::default())
        .await
        .expect("online install");
    std::fs::remove_dir_all(target.path().join("node_modules")).expect("clean");

    let offline = Installer::with_registry(&stub.base, home.path()).offline();
    let report = offline
        .install(target.path(), &spec, &InstallOptions::default())
        .await
        .expect("offline install");
    assert_eq!(report.version_of("a"), Some("1.0.0"));
    assert!(target.path().join("node_modules/a/index.js").is_file());
}
