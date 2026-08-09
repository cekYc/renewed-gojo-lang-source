use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};

use crate::project::{self, ManifestData, ManifestDependency, Project};

const LOCK_FILE_NAME: &str = "zet.lock";
const PACKAGE_MARKER: &str = ".zet-package";

#[derive(Debug, Clone)]
struct LockedPackage {
    name: String,
    git: String,
    requirement: String,
    version: String,
    commit: String,
    checksum: String,
}

#[derive(Debug, Clone)]
struct SelectedRevision {
    version: Option<Version>,
    commit: String,
}

#[derive(Debug, Clone)]
pub struct PackageImport {
    pub entry: PathBuf,
    pub source_root: PathBuf,
}

struct Resolver {
    packages_dir: PathBuf,
    locked: HashMap<String, LockedPackage>,
    resolved: HashMap<String, LockedPackage>,
    resolving: HashSet<String>,
    force_all: bool,
    force_names: HashSet<String>,
}

pub fn add(project: &Project, specification: &str) -> Result<(), String> {
    let manifest_path = project_manifest_path(project)?;
    let (git, requested_version) = parse_package_spec(specification)?;
    let mirror = ensure_mirror(&git, true, None)?;
    let requirement = requested_version.as_deref().unwrap_or("*");
    let selected = select_revision(&mirror, requirement)?;
    let manifest = manifest_at_revision(&mirror, &selected.commit)?;
    project::validate_dependency_name(&manifest.name)?;
    validate_selected_manifest(&manifest, &selected, requirement)?;

    let dependency = ManifestDependency {
        name: manifest.name.clone(),
        git,
        version: manifest.version.clone(),
    };
    project::set_dependency(manifest_path, &dependency)?;
    let refreshed = project::load_project(manifest_path)?;
    install_with_mode(&refreshed, false, HashSet::from([dependency.name.clone()]))?;
    println!("Paket eklendi: {} v{}", dependency.name, dependency.version);
    Ok(())
}

pub fn remove(project: &Project, name: &str) -> Result<(), String> {
    let manifest_path = project_manifest_path(project)?;
    if !project::remove_dependency(manifest_path, name)? {
        return Err(format!("Bagimlilik bulunamadi: {name}"));
    }
    let refreshed = project::load_project(manifest_path)?;
    install_with_mode(&refreshed, false, HashSet::new())?;
    println!("Paket kaldirildi: {name}");
    Ok(())
}

pub fn install(project: &Project) -> Result<(), String> {
    install_with_mode(project, false, HashSet::new())
}

pub fn update(project: &Project, target: Option<&str>) -> Result<(), String> {
    let manifest_path = project_manifest_path(project)?;
    if project.dependencies.is_empty() {
        return Err("Guncellenecek bagimlilik yok.".to_string());
    }

    let targets: Vec<ManifestDependency> = match target {
        Some(name) => vec![project
            .dependencies
            .iter()
            .find(|dependency| dependency.name == name)
            .cloned()
            .ok_or_else(|| format!("Bagimlilik bulunamadi: {name}"))?],
        None => project.dependencies.clone(),
    };

    let mut force_names = HashSet::new();
    for dependency in targets {
        let mirror = ensure_mirror(&dependency.git, true, None)?;
        let selected = select_revision(&mirror, "*")?;
        let manifest = manifest_at_revision(&mirror, &selected.commit)?;
        if manifest.name != dependency.name {
            return Err(format!(
                "Paket adi degisti: zet.toml '{}' bekliyor, depo '{}' bildiriyor.",
                dependency.name, manifest.name
            ));
        }
        validate_selected_manifest(&manifest, &selected, "*")?;
        project::set_dependency(
            manifest_path,
            &ManifestDependency {
                name: dependency.name.clone(),
                git: dependency.git,
                version: manifest.version,
            },
        )?;
        force_names.insert(dependency.name);
    }

    let refreshed = project::load_project(manifest_path)?;
    install_with_mode(&refreshed, target.is_none(), force_names)?;
    Ok(())
}

pub fn import_map(project: &Project) -> Result<HashMap<String, PackageImport>, String> {
    if project.dependencies.is_empty() {
        return Ok(HashMap::new());
    }
    let lock_path = project.root.join(LOCK_FILE_NAME);
    let locked = read_lock_file(&lock_path)?;
    if locked.is_empty() {
        return Err("Paket kilidi bulunamadi. Once 'zet install' calistirin.".to_string());
    }

    for dependency in &project.dependencies {
        if !locked.iter().any(|package| package.name == dependency.name) {
            return Err(format!(
                "{} zet.lock icinde yok. 'zet install' calistirin.",
                dependency.name
            ));
        }
    }

    let packages_dir = project.root.join(".zet").join("packages");
    let mut imports = HashMap::new();
    for package in locked {
        let package_dir = packages_dir.join(&package.name);
        let marker = read_marker(&package_dir.join(PACKAGE_MARKER))?;
        if marker.0 != package.commit || marker.1 != package.checksum {
            return Err(format!(
                "{} paketi eksik veya kilitle uyusmuyor. 'zet install' calistirin.",
                package.name
            ));
        }
        let manifest_path = package_dir.join("zet.toml");
        let content = fs::read_to_string(&manifest_path).map_err(|error| {
            format!(
                "Paket manifesti okunamadi ({}): {error}",
                manifest_path.display()
            )
        })?;
        let manifest = project::parse_manifest(&content)?;
        let entry = package_dir.join(&manifest.entry);
        if !entry.is_file() {
            return Err(format!(
                "{} paket girisi bulunamadi: {}",
                package.name,
                entry.display()
            ));
        }
        let source_root = entry.parent().unwrap_or(&package_dir).to_path_buf();
        imports.insert(package.name, PackageImport { entry, source_root });
    }
    Ok(imports)
}

fn install_with_mode(
    project: &Project,
    force_all: bool,
    force_names: HashSet<String>,
) -> Result<(), String> {
    let _ = project_manifest_path(project)?;
    let packages_dir = project.root.join(".zet").join("packages");
    fs::create_dir_all(&packages_dir)
        .map_err(|error| format!("Paket dizini olusturulamadi: {error}"))?;
    let lock_path = project.root.join(LOCK_FILE_NAME);
    let locked = if lock_path.is_file() {
        read_lock_file(&lock_path)?
            .into_iter()
            .map(|package| (package.name.clone(), package))
            .collect()
    } else {
        HashMap::new()
    };

    let mut resolver = Resolver {
        packages_dir,
        locked,
        resolved: HashMap::new(),
        resolving: HashSet::new(),
        force_all,
        force_names,
    };
    for dependency in &project.dependencies {
        resolver.resolve(dependency)?;
    }
    resolver.clean_unused_packages()?;

    let mut packages: Vec<LockedPackage> = resolver.resolved.into_values().collect();
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    write_lock_file(&lock_path, &packages)?;
    println!(
        "{} paket kuruldu; {} guncellendi.",
        packages.len(),
        LOCK_FILE_NAME
    );
    Ok(())
}

impl Resolver {
    fn resolve(&mut self, dependency: &ManifestDependency) -> Result<(), String> {
        if let Some(existing) = self.resolved.get(&dependency.name) {
            if existing.git != dependency.git
                || !version_matches(&dependency.version, &existing.version)?
            {
                return Err(format!(
                    "Bagimlilik cakismasi: {} icin {} v{} zaten secildi; {} isteniyor.",
                    dependency.name, existing.git, existing.version, dependency.version
                ));
            }
            return Ok(());
        }
        if !self.resolving.insert(dependency.name.clone()) {
            return Err(format!("Dongusel bagimlilik: {}", dependency.name));
        }

        let force = self.force_all || self.force_names.contains(&dependency.name);
        let locked = self.locked.get(&dependency.name).filter(|package| {
            !force
                && package.git == dependency.git
                && version_matches(&dependency.version, &package.version).unwrap_or(false)
        });
        let (mirror, selected, expected_checksum) = if let Some(package) = locked {
            let mirror = ensure_mirror(&dependency.git, false, Some(&package.commit))?;
            (
                mirror,
                SelectedRevision {
                    version: Version::parse(&package.version).ok(),
                    commit: package.commit.clone(),
                },
                Some(package.checksum.clone()),
            )
        } else {
            let mirror = ensure_mirror(&dependency.git, true, None)?;
            let selected = select_revision(&mirror, &dependency.version)?;
            (mirror, selected, None)
        };

        let manifest = manifest_at_revision(&mirror, &selected.commit)?;
        if manifest.name != dependency.name {
            return Err(format!(
                "Paket adi uyusmuyor: bagimlilik '{}' fakat depo '{}'.",
                dependency.name, manifest.name
            ));
        }
        validate_selected_manifest(&manifest, &selected, &dependency.version)?;
        let checksum = install_checkout(
            &mirror,
            &selected.commit,
            &dependency.name,
            &self.packages_dir,
        )?;
        if let Some(expected) = expected_checksum {
            if checksum != expected {
                return Err(format!(
                    "Butunluk hatasi: {} icin kilit {} fakat checkout {}.",
                    dependency.name, expected, checksum
                ));
            }
        }

        self.resolved.insert(
            dependency.name.clone(),
            LockedPackage {
                name: dependency.name.clone(),
                git: dependency.git.clone(),
                requirement: dependency.version.clone(),
                version: manifest.version.clone(),
                commit: selected.commit.clone(),
                checksum,
            },
        );

        for child in &manifest.dependencies {
            self.resolve(child)?;
        }
        self.resolving.remove(&dependency.name);
        Ok(())
    }

    fn clean_unused_packages(&self) -> Result<(), String> {
        for entry in fs::read_dir(&self.packages_dir)
            .map_err(|error| format!("Paket dizini okunamadi: {error}"))?
        {
            let entry = entry.map_err(|error| format!("Paket girdisi okunamadi: {error}"))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || self.resolved.contains_key(&name) {
                continue;
            }
            safe_remove_dir(&entry.path(), &self.packages_dir)?;
        }
        Ok(())
    }
}

fn parse_package_spec(specification: &str) -> Result<(String, Option<String>), String> {
    let (source, version) = specification
        .rsplit_once('@')
        .map(|(source, version)| (source, Some(version.to_string())))
        .unwrap_or((specification, None));
    if source.is_empty() || version.as_deref() == Some("") {
        return Err("Paket bicimi: sahip/depo veya sahip/depo@surum".to_string());
    }
    if let Some(version) = &version {
        parse_requirement(version)?;
    }

    let git = if source.contains("://") {
        source.trim_end_matches('/').to_string()
    } else {
        let parts: Vec<&str> = source.split('/').collect();
        if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
            return Err("Paket bicimi: sahip/depo veya sahip/depo@surum".to_string());
        }
        format!("https://github.com/{}/{}.git", parts[0], parts[1])
    };
    Ok((git, version))
}

fn cache_root() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("ZET_CACHE_DIR") {
        return Ok(PathBuf::from(path));
    }
    if cfg!(windows) {
        return env::var_os("LOCALAPPDATA")
            .map(|path| PathBuf::from(path).join("Zet").join("cache"))
            .ok_or_else(|| "LOCALAPPDATA tanimli degil; ZET_CACHE_DIR ayarlayin.".to_string());
    }
    if let Some(path) = env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(path).join("zet"));
    }
    env::var_os("HOME")
        .map(|path| PathBuf::from(path).join(".cache").join("zet"))
        .ok_or_else(|| "HOME tanimli degil; ZET_CACHE_DIR ayarlayin.".to_string())
}

fn ensure_mirror(git: &str, fetch: bool, required_commit: Option<&str>) -> Result<PathBuf, String> {
    let mut hasher = Sha256::new();
    hasher.update(git.as_bytes());
    let key = format!("{:x}", hasher.finalize());
    let mirror = cache_root()?.join("git").join(format!("{key}.git"));
    if !mirror.exists() {
        fs::create_dir_all(mirror.parent().unwrap_or(Path::new(".")))
            .map_err(|error| format!("Paket onbellegi olusturulamadi: {error}"))?;
        let mut command = Command::new("git");
        command
            .arg("-c")
            .arg("core.autocrlf=false")
            .arg("clone")
            .arg("--mirror")
            .arg("--quiet")
            .arg(git)
            .arg(&mirror);
        run_command(command, &format!("Git deposu klonlanamadi: {git}"))?;
    }

    let commit_missing = required_commit
        .map(|commit| !mirror_has_commit(&mirror, commit))
        .unwrap_or(false);
    if fetch || commit_missing {
        let mut command = Command::new("git");
        command
            .arg("--git-dir")
            .arg(&mirror)
            .arg("fetch")
            .arg("--quiet")
            .arg("--prune")
            .arg("--tags")
            .arg("origin");
        run_command(command, &format!("Git deposu guncellenemedi: {git}"))?;
    }
    if let Some(commit) = required_commit {
        if !mirror_has_commit(&mirror, commit) {
            return Err(format!("Kilitli commit depoda bulunamadi: {commit}"));
        }
    }
    Ok(mirror)
}

fn mirror_has_commit(mirror: &Path, commit: &str) -> bool {
    Command::new("git")
        .arg("--git-dir")
        .arg(mirror)
        .arg("cat-file")
        .arg("-e")
        .arg(format!("{commit}^{{commit}}"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn select_revision(mirror: &Path, requirement: &str) -> Result<SelectedRevision, String> {
    let output = git_output(
        mirror,
        &["for-each-ref", "--format=%(refname:short)", "refs/tags"],
        "Git etiketleri okunamadi",
    )?;
    let mut versions = Vec::new();
    for tag in output.lines().map(str::trim).filter(|tag| !tag.is_empty()) {
        let raw = tag.strip_prefix('v').unwrap_or(tag);
        if let Ok(version) = Version::parse(raw) {
            if version_matches(requirement, &version.to_string())? {
                versions.push((version, tag.to_string()));
            }
        }
    }
    versions.sort_by(|left, right| left.0.cmp(&right.0));

    if let Some((version, tag)) = versions.pop() {
        let commit = git_output(
            mirror,
            &["rev-list", "-n", "1", &tag],
            "Paket etiketi cozumlenemedi",
        )?
        .trim()
        .to_string();
        return Ok(SelectedRevision {
            version: Some(version),
            commit,
        });
    }
    if requirement == "*" {
        Err("Paket deposunda SemVer etiketi yok (vX.Y.Z veya X.Y.Z bekleniyor).".to_string())
    } else {
        Err(format!(
            "SemVer kosuluyla eslesen paket surumu yok: {requirement}"
        ))
    }
}

fn manifest_at_revision(mirror: &Path, commit: &str) -> Result<ManifestData, String> {
    let specification = format!("{commit}:zet.toml");
    let content = git_output(
        mirror,
        &["show", &specification],
        "Paket deposunda zet.toml bulunamadi",
    )?;
    project::parse_manifest(&content)
}

fn validate_selected_manifest(
    manifest: &ManifestData,
    selected: &SelectedRevision,
    requirement: &str,
) -> Result<(), String> {
    let manifest_version = Version::parse(&manifest.version)
        .map_err(|error| format!("Paket surumu SemVer degil ({}): {error}", manifest.version))?;
    if !version_matches(requirement, &manifest.version)? {
        return Err(format!(
            "Paket manifest surumu {} istenen {} kosulunu karsilamiyor.",
            manifest.version, requirement
        ));
    }
    if let Some(tag_version) = &selected.version {
        if tag_version != &manifest_version {
            return Err(format!(
                "Etiket v{} fakat paket manifesti {} bildiriyor.",
                tag_version, manifest.version
            ));
        }
    }
    Ok(())
}

fn install_checkout(
    mirror: &Path,
    commit: &str,
    name: &str,
    packages_dir: &Path,
) -> Result<String, String> {
    project::validate_dependency_name(name)?;
    let destination = packages_dir.join(name);
    let staging = packages_dir.join(format!(".{name}.tmp-{}", std::process::id()));
    if staging.exists() {
        safe_remove_dir(&staging, packages_dir)?;
    }

    let mut clone = Command::new("git");
    clone
        .arg("-c")
        .arg("core.autocrlf=false")
        .arg("clone")
        .arg("--quiet")
        .arg("--no-checkout")
        .arg(mirror)
        .arg(&staging);
    run_command(clone, &format!("{} paketi checkout edilemedi", name))?;

    let mut checkout = Command::new("git");
    checkout
        .arg("-C")
        .arg(&staging)
        .arg("-c")
        .arg("core.autocrlf=false")
        .arg("checkout")
        .arg("--quiet")
        .arg("--detach")
        .arg(commit);
    run_command(checkout, &format!("{} commit'i checkout edilemedi", name))?;

    let git_dir = staging.join(".git");
    if git_dir.exists() {
        safe_remove_dir(&git_dir, &staging)?;
    }
    let checksum = hash_package(&staging)?;
    fs::write(
        staging.join(PACKAGE_MARKER),
        format!("commit={commit}\nchecksum={checksum}\n"),
    )
    .map_err(|error| format!("Paket isareti yazilamadi: {error}"))?;

    if destination.exists() {
        safe_remove_dir(&destination, packages_dir)?;
    }
    fs::rename(&staging, &destination).map_err(|error| {
        format!(
            "Paket etkinlestirilemedi ({}): {error}",
            destination.display()
        )
    })?;
    Ok(checksum)
}

fn hash_package(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_package_files(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    for (relative, path) in files {
        let content = fs::read(&path)
            .map_err(|error| format!("Paket dosyasi okunamadi ({}): {error}", path.display()))?;
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        hasher.update((content.len() as u64).to_le_bytes());
        hasher.update(&content);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn collect_package_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("Paket dizini okunamadi ({}): {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("Paket girdisi okunamadi: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Paket dosya tipi okunamadi: {error}"))?;
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" || name == PACKAGE_MARKER {
            continue;
        }
        if file_type.is_symlink() {
            return Err(format!(
                "Paket sembolik baglanti iceremez: {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            collect_package_files(root, &path, files)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("Paket yolu hesaplanamadi: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((relative, path));
        }
    }
    Ok(())
}

fn write_lock_file(path: &Path, packages: &[LockedPackage]) -> Result<(), String> {
    let mut content = "lock_version = 1\n".to_string();
    for package in packages {
        content.push_str(&format!(
            "\n[[package]]\nname = \"{}\"\ngit = \"{}\"\nrequirement = \"{}\"\nversion = \"{}\"\ncommit = \"{}\"\nchecksum = \"{}\"\n",
            package.name,
            package.git,
            package.requirement,
            package.version,
            package.commit,
            package.checksum
        ));
    }
    fs::write(path, content).map_err(|error| format!("{} yazilamadi: {error}", path.display()))
}

fn read_lock_file(path: &Path) -> Result<Vec<LockedPackage>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("{} okunamadi: {error}", path.display()))?;
    let mut packages = Vec::new();
    let mut current = HashMap::new();
    let mut lock_version = None;
    for line in content.lines().map(str::trim) {
        if line == "[[package]]" {
            if !current.is_empty() {
                packages.push(lock_package_from_values(&current)?);
                current.clear();
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("lock_version") {
            let value = value
                .trim()
                .strip_prefix('=')
                .map(str::trim)
                .ok_or_else(|| "zet.lock lock_version satiri gecersiz.".to_string())?;
            lock_version = Some(value.to_string());
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if let Some(value) = quoted_value(value.trim()) {
                current.insert(key.trim().to_string(), value);
            }
        }
    }
    if !current.is_empty() {
        packages.push(lock_package_from_values(&current)?);
    }
    if lock_version.as_deref() != Some("1") {
        return Err(format!(
            "Desteklenmeyen zet.lock surumu: {}",
            lock_version.as_deref().unwrap_or("eksik")
        ));
    }
    let mut names = HashSet::new();
    for package in &packages {
        if !names.insert(&package.name) {
            return Err(format!("zet.lock icinde yinelenen paket: {}", package.name));
        }
    }
    Ok(packages)
}

fn lock_package_from_values(values: &HashMap<String, String>) -> Result<LockedPackage, String> {
    let get = |key: &str| {
        values
            .get(key)
            .cloned()
            .ok_or_else(|| format!("zet.lock paket kaydinda '{key}' eksik."))
    };
    Ok(LockedPackage {
        name: get("name")?,
        git: get("git")?,
        requirement: get("requirement")?,
        version: get("version")?,
        commit: get("commit")?,
        checksum: get("checksum")?,
    })
}

fn read_marker(path: &Path) -> Result<(String, String), String> {
    let content = fs::read_to_string(path)
        .map_err(|_| format!("Paket kurulumu eksik: {}", path.display()))?;
    let mut commit = None;
    let mut checksum = None;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("commit=") {
            commit = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("checksum=") {
            checksum = Some(value.to_string());
        }
    }
    Ok((
        commit.ok_or_else(|| "Paket isaretinde commit eksik.".to_string())?,
        checksum.ok_or_else(|| "Paket isaretinde checksum eksik.".to_string())?,
    ))
}

fn version_matches(requirement: &str, version: &str) -> Result<bool, String> {
    let version = Version::parse(version)
        .map_err(|error| format!("Gecersiz SemVer surumu '{version}': {error}"))?;
    if requirement == "*" {
        return Ok(true);
    }
    if let Ok(exact) = Version::parse(requirement) {
        return Ok(exact == version);
    }
    Ok(parse_requirement(requirement)?.matches(&version))
}

fn parse_requirement(requirement: &str) -> Result<VersionReq, String> {
    if let Ok(version) = Version::parse(requirement) {
        return VersionReq::parse(&format!("={version}"))
            .map_err(|error| format!("Gecersiz SemVer kosulu '{requirement}': {error}"));
    }
    VersionReq::parse(requirement)
        .map_err(|error| format!("Gecersiz SemVer kosulu '{requirement}': {error}"))
}

fn project_manifest_path(project: &Project) -> Result<&Path, String> {
    project
        .manifest_path
        .as_deref()
        .ok_or_else(|| "Paket komutlari icin zet.toml bulunan bir proje gerekir.".to_string())
}

fn safe_remove_dir(path: &Path, expected_parent: &Path) -> Result<(), String> {
    if path.parent() != Some(expected_parent) || path == expected_parent {
        return Err(format!(
            "Guvenli olmayan paket silme hedefi: {}",
            path.display()
        ));
    }
    fs::remove_dir_all(path)
        .map_err(|error| format!("Paket dizini silinemedi ({}): {error}", path.display()))
}

fn quoted_value(value: &str) -> Option<String> {
    let value = value.trim();
    let inner = value.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

fn git_output(mirror: &Path, args: &[&str], message: &str) -> Result<String, String> {
    let mut command = Command::new("git");
    command.arg("--git-dir").arg(mirror);
    command.args(args);
    let output = command
        .output()
        .map_err(|error| format!("Git baslatilamadi: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{message}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("Git ciktisi UTF-8 degil: {error}"))
}

fn run_command(mut command: Command, message: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("Git baslatilamadi: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{message}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
