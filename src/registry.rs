use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::project::{self, Project};

const REGISTRY_SCHEMA: u32 = 1;
const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/cekYc/zet-lang-source/main/registry/index.json";
const DEFAULT_ISSUES_API: &str = "https://api.github.com/repos/cekYc/zet-lang-source/issues";
const USER_AGENT: &str = "zet-compiler/0.6.5";

#[derive(Debug, Deserialize)]
struct RegistryIndex {
    schema: u32,
    #[serde(default)]
    packages: BTreeMap<String, RegistryPackage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryPackage {
    pub git: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub latest: String,
    #[serde(default)]
    pub versions: BTreeMap<String, RegistryRelease>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryRelease {
    pub commit: String,
    pub checksum: String,
}

#[derive(Debug, Clone)]
pub struct RegistrySelection {
    pub name: String,
    pub git: String,
    pub version: String,
    pub commit: String,
    pub checksum: String,
}

#[derive(Serialize)]
struct PublishRequest<'a> {
    name: &'a str,
    version: &'a str,
    description: &'a str,
    git: &'a str,
    commit: &'a str,
}

#[derive(Deserialize)]
struct CreatedIssue {
    html_url: String,
}

pub fn resolve(name: &str) -> Result<RegistryPackage, String> {
    project::validate_dependency_name(name)?;
    let registry = fetch_registry()?;
    registry
        .packages
        .get(name)
        .cloned()
        .ok_or_else(|| format!("Paket merkezi kayitta bulunamadi: {name}"))
}

pub fn select(name: &str, requirement: &str) -> Result<RegistrySelection, String> {
    let package = resolve(name)?;
    let requirement = parse_requirement(requirement)?;
    let mut matches = Vec::new();
    for (raw_version, release) in &package.versions {
        let version = Version::parse(raw_version).map_err(|error| {
            format!("Registry paket surumu gecersiz ({name} {raw_version}): {error}")
        })?;
        if requirement.matches(&version) {
            matches.push((version, release));
        }
    }
    matches.sort_by(|left, right| left.0.cmp(&right.0));
    let (version, release) = matches
        .pop()
        .ok_or_else(|| format!("Registry paketinde istenen kosulla eslesen surum yok: {name}"))?;
    Ok(RegistrySelection {
        name: name.to_string(),
        git: package.git,
        version: version.to_string(),
        commit: release.commit.clone(),
        checksum: release.checksum.clone(),
    })
}

pub fn search(query: &str) -> Result<(), String> {
    let registry = fetch_registry()?;
    let query = query.to_ascii_lowercase();
    let matches: Vec<(&String, &RegistryPackage)> = registry
        .packages
        .iter()
        .filter(|(name, package)| {
            query.is_empty()
                || name.to_ascii_lowercase().contains(&query)
                || package.description.to_ascii_lowercase().contains(&query)
        })
        .take(50)
        .collect();

    if matches.is_empty() {
        println!("Eslesen paket bulunamadi.");
        return Ok(());
    }

    println!("Zet Registry - {} paket", matches.len());
    for (name, package) in matches {
        let version = if package.latest.is_empty() {
            "surum yok".to_string()
        } else {
            format!("v{}", package.latest)
        };
        if package.description.is_empty() {
            println!("  {name} {version} - {}", package.git);
        } else {
            println!("  {name} {version} - {}", package.description);
            println!("    {} ({})", package.git, package.owner);
        }
    }
    Ok(())
}

pub fn publish(project: &Project, dry_run: bool) -> Result<(), String> {
    let manifest_path = project
        .manifest_path
        .as_deref()
        .ok_or_else(|| "zet publish icin zet.toml bulunan bir proje gerekir.".to_string())?;
    let manifest_content = fs::read_to_string(manifest_path)
        .map_err(|error| format!("zet.toml okunamadi: {error}"))?;
    let manifest = project::parse_manifest(&manifest_content)?;
    project::validate_dependency_name(&manifest.name)?;
    Version::parse(&manifest.version)
        .map_err(|error| format!("Paket surumu SemVer degil ({}): {error}", manifest.version))?;
    validate_description(&manifest.description)?;

    let root = project.root.canonicalize().map_err(|error| {
        format!(
            "Proje dizini cozumlenemedi ({}): {error}",
            project.root.display()
        )
    })?;
    let git_root = PathBuf::from(git_output(&root, &["rev-parse", "--show-toplevel"])?)
        .canonicalize()
        .map_err(|error| format!("Git depo koku cozumlenemedi: {error}"))?;
    if git_root != root {
        return Err("zet.toml Git deposunun kokunde olmalidir.".to_string());
    }

    let status = git_output(
        &root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    if !status.is_empty() {
        return Err("Calisma agaci temiz degil. Once degisiklikleri commit edin.".to_string());
    }

    let entry = manifest.entry.to_string_lossy().replace('\\', "/");
    git_output(
        &root,
        &["ls-files", "--error-unmatch", "--", "zet.toml", &entry],
    )
    .map_err(|_| "zet.toml ve paket entry dosyasi Git tarafindan izlenmelidir.".to_string())?;
    if !root.join(&manifest.entry).is_file() {
        return Err(format!("Paket girisi bulunamadi: {entry}"));
    }

    let remote = git_output(&root, &["remote", "get-url", "origin"])?;
    let git = normalize_github_remote(&remote)?;
    let commit = git_output(&root, &["rev-parse", "HEAD"])?;
    let preferred_tag = format!("v{}", manifest.version);
    let plain_tag = manifest.version.clone();
    let selected_tag = select_publish_tag(&root, &preferred_tag, &plain_tag, &commit)?;

    if dry_run {
        println!("Yayin dogrulamasi basarili (dry-run).");
        println!("  Paket: {} v{}", manifest.name, manifest.version);
        println!("  Depo: {git}");
        println!("  Commit: {commit}");
        println!("  Etiket: {selected_tag}");
        println!("Etiket push edilmedi ve kayit istegi acilmadi.");
        return Ok(());
    }

    if local_tag_commit(&root, &selected_tag)?.is_none() {
        git_run(
            &root,
            &[
                "tag",
                "-a",
                &selected_tag,
                "-m",
                &format!("{} v{}", manifest.name, manifest.version),
            ],
        )?;
    }
    git_run(
        &root,
        &["push", "origin", &format!("refs/tags/{selected_tag}")],
    )?;

    let request = PublishRequest {
        name: &manifest.name,
        version: &manifest.version,
        description: &manifest.description,
        git: &git,
        commit: &commit,
    };
    let request_json = serde_json::to_string(&request)
        .map_err(|error| format!("Kayit istegi olusturulamadi: {error}"))?;
    let title = format!("[zet-publish] {} v{}", manifest.name, manifest.version);
    let body = format!(
        "Zet Registry publish request. Bu kayit otomatik olarak dogrulanacaktir.\n\n<!-- zet-publish-v1\n{request_json}\n-->"
    );
    let issue_url = create_registry_issue(&title, &body)?;

    println!("Paket etiketi yayinlandi: {selected_tag}");
    println!("Kayit istegi acildi: {issue_url}");
    println!("GitHub Actions dogrulamasindan sonra paket ada gore kurulabilir.");
    Ok(())
}

fn fetch_registry() -> Result<RegistryIndex, String> {
    let content = if let Some(path) = env::var_os("ZET_REGISTRY_FILE") {
        fs::read_to_string(PathBuf::from(path))
            .map_err(|error| format!("Yerel registry okunamadi: {error}"))?
    } else {
        let url = env::var("ZET_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string());
        http_get(&url)?
    };
    let registry: RegistryIndex = serde_json::from_str(&content)
        .map_err(|error| format!("Registry JSON gecersiz: {error}"))?;
    if registry.schema != REGISTRY_SCHEMA {
        return Err(format!(
            "Desteklenmeyen registry semasi: {}",
            registry.schema
        ));
    }
    for (name, package) in &registry.packages {
        project::validate_dependency_name(name)?;
        if package.git.is_empty() {
            return Err(format!("Registry paketinde git alani eksik: {name}"));
        }
        if package.owner.trim().is_empty() {
            return Err(format!("Registry paketinde owner alani eksik: {name}"));
        }
        if !package.latest.is_empty() && !package.versions.contains_key(&package.latest) {
            return Err(format!(
                "Registry paketinin latest surumu bulunamadi: {name} {}",
                package.latest
            ));
        }
        for (version, release) in &package.versions {
            Version::parse(version).map_err(|error| {
                format!("Registry paket surumu gecersiz ({name} {version}): {error}")
            })?;
            if release.commit.len() != 40
                || !release
                    .commit
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
                || !release.checksum.starts_with("sha256:")
                || release.checksum.len() != 71
                || !release.checksum["sha256:".len()..]
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                return Err(format!("Registry yayin kaydi gecersiz: {name} {version}"));
            }
        }
    }
    Ok(registry)
}

fn parse_requirement(requirement: &str) -> Result<VersionReq, String> {
    if requirement == "*" {
        return VersionReq::parse("*").map_err(|error| error.to_string());
    }
    if let Ok(version) = Version::parse(requirement) {
        return VersionReq::parse(&format!("={version}"))
            .map_err(|error| format!("Gecersiz SemVer kosulu '{requirement}': {error}"));
    }
    VersionReq::parse(requirement)
        .map_err(|error| format!("Gecersiz SemVer kosulu '{requirement}': {error}"))
}

fn http_get(url: &str) -> Result<String, String> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| format!("Registry runtime baslatilamadi: {error}"))?;
    runtime.block_on(async {
        let response = reqwest::Client::new()
            .get(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| format!("Registry indirilemedi: {error}"))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| format!("Registry yaniti okunamadi: {error}"))?;
        if !status.is_success() {
            return Err(format!("Registry HTTP hatasi: {status}"));
        }
        Ok(body)
    })
}

fn create_registry_issue(title: &str, body: &str) -> Result<String, String> {
    let token = registry_token()?;
    let url =
        env::var("ZET_REGISTRY_ISSUES_API").unwrap_or_else(|_| DEFAULT_ISSUES_API.to_string());
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| format!("Yayin runtime baslatilamadi: {error}"))?;
    runtime.block_on(async {
        let response = reqwest::Client::new()
            .post(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .bearer_auth(token)
            .json(&json!({ "title": title, "body": body }))
            .send()
            .await
            .map_err(|error| format!("Kayit istegi gonderilemedi: {error}"))?;
        let status = response.status();
        let content = response
            .text()
            .await
            .map_err(|error| format!("GitHub yaniti okunamadi: {error}"))?;
        if !status.is_success() {
            let summary: String = content.chars().take(300).collect();
            return Err(format!(
                "GitHub kayit istegini reddetti ({status}): {summary}"
            ));
        }
        let issue: CreatedIssue = serde_json::from_str(&content)
            .map_err(|error| format!("GitHub yaniti gecersiz: {error}"))?;
        Ok(issue.html_url)
    })
}

fn registry_token() -> Result<String, String> {
    for key in ["ZET_REGISTRY_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(token) = env::var(key) {
            if !token.trim().is_empty() {
                return Ok(token);
            }
        }
    }
    let output = Command::new("gh").args(["auth", "token"]).output();
    if let Ok(output) = output {
        if output.status.success() {
            let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }
    }
    Err(
        "Kayit istegi icin 'gh auth login' calistirin veya ZET_REGISTRY_TOKEN ayarlayin."
            .to_string(),
    )
}

fn validate_description(description: &str) -> Result<(), String> {
    if description.chars().count() > 160
        || description.contains("-->")
        || description
            .chars()
            .any(|character| character.is_control() && character != '\t')
    {
        return Err("Paket aciklamasi en fazla 160 yazdirilabilir karakter olmalidir.".to_string());
    }
    Ok(())
}

fn normalize_github_remote(remote: &str) -> Result<String, String> {
    let remote = remote.trim().trim_end_matches('/');
    let path = if let Some(path) = remote.strip_prefix("https://github.com/") {
        path
    } else if let Some(path) = remote.strip_prefix("git@github.com:") {
        path
    } else if let Some(path) = remote.strip_prefix("ssh://git@github.com/") {
        path
    } else {
        return Err("v0.6.5 registry yalnizca GitHub origin depolarini kabul eder.".to_string());
    };
    let path = path.strip_suffix(".git").unwrap_or(path);
    let parts: Vec<&str> = path.split('/').collect();
    let valid = parts.len() == 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
        });
    if !valid {
        return Err(format!("GitHub origin URL'si gecersiz: {remote}"));
    }
    Ok(format!("https://github.com/{}/{}.git", parts[0], parts[1]))
}

fn select_publish_tag(
    root: &Path,
    preferred: &str,
    plain: &str,
    commit: &str,
) -> Result<String, String> {
    for tag in [preferred, plain] {
        if let Some(tag_commit) = local_tag_commit(root, tag)? {
            if tag_commit != commit {
                return Err(format!(
                    "{tag} etiketi mevcut fakat HEAD commit'ini gostermiyor. Surumu artirin."
                ));
            }
            return Ok(tag.to_string());
        }
    }
    Ok(preferred.to_string())
}

fn local_tag_commit(root: &Path, tag: &str) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-list", "-n", "1", &format!("refs/tags/{tag}")])
        .output()
        .map_err(|error| format!("Git baslatilamadi: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("Git baslatilamadi: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Git komutu basarisiz: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("Git ciktisi UTF-8 degil: {error}"))
}

fn git_run(root: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("Git baslatilamadi: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Git komutu basarisiz: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
