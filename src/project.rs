use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const DEFAULT_ENTRY: &str = "src/main.zt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestDependency {
    pub name: String,
    pub git: String,
    pub version: String,
    pub registry: bool,
}

#[derive(Debug, Clone)]
pub struct ManifestData {
    pub name: String,
    pub version: String,
    pub description: String,
    pub entry: PathBuf,
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub version: String,
    pub root: PathBuf,
    pub source: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Debug, Clone)]
pub struct BuildLayout {
    pub runtime_dir: PathBuf,
    pub generated_source: PathBuf,
    pub target_dir: PathBuf,
    pub bin_dir: PathBuf,
}

pub fn create_project(requested_path: &Path) -> Result<Project, String> {
    let target = absolutize(requested_path)?;
    if target.exists() {
        return Err(format!("Hedef zaten mevcut: {}", target.display()));
    }

    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Proje adi belirlenemedi.".to_string())?
        .to_string();
    validate_project_name(&name)?;

    let source_dir = target.join("src");
    fs::create_dir_all(&source_dir)
        .map_err(|error| format!("Proje klasoru olusturulamadi: {error}"))?;

    let manifest = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\ndescription = \"Zet project {name}\"\nentry = \"{DEFAULT_ENTRY}\"\n\n[dependencies]\n"
    );
    let main_source =
        format!("nondet fn main() -> Void {{\n    println(\"Merhaba, {name}!\")\n}}\n");
    let manifest_path = target.join("zet.toml");

    fs::write(&manifest_path, manifest).map_err(|error| format!("zet.toml yazilamadi: {error}"))?;
    fs::write(source_dir.join("main.zt"), main_source)
        .map_err(|error| format!("Ana kaynak dosyasi yazilamadi: {error}"))?;
    fs::write(target.join(".gitignore"), "/.zet/\n")
        .map_err(|error| format!(".gitignore yazilamadi: {error}"))?;

    Ok(Project {
        name,
        version: "0.1.0".to_string(),
        root: target.clone(),
        source: target.join(DEFAULT_ENTRY),
        manifest_path: Some(manifest_path),
        dependencies: Vec::new(),
    })
}

pub fn resolve_project(explicit_source: Option<&Path>) -> Result<Project, String> {
    if let Some(source) = explicit_source {
        let source = absolutize(source)?;
        if !source.is_file() {
            return Err(format!("Kaynak dosyasi bulunamadi: {}", source.display()));
        }

        if let Some(manifest) = find_manifest(source.parent().unwrap_or(Path::new("."))) {
            let mut project = load_project(&manifest)?;
            project.source = source;
            return Ok(project);
        }

        let root = source.parent().unwrap_or(Path::new(".")).to_path_buf();
        let name = source
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("app")
            .to_string();
        return Ok(Project {
            name,
            version: "0.0.0".to_string(),
            root,
            source,
            manifest_path: None,
            dependencies: Vec::new(),
        });
    }

    let current =
        env::current_dir().map_err(|error| format!("Calisma dizini okunamadi: {error}"))?;
    let manifest = find_manifest(&current).ok_or_else(|| {
        "zet.toml bulunamadi. Bir proje icinde calistirin veya bir .zt dosyasi belirtin."
            .to_string()
    })?;
    let project = load_project(&manifest)?;
    if !project.source.is_file() {
        return Err(format!(
            "Proje giris dosyasi bulunamadi: {}",
            project.source.display()
        ));
    }
    Ok(project)
}

pub fn resolve_manifest_project() -> Result<Project, String> {
    let current =
        env::current_dir().map_err(|error| format!("Calisma dizini okunamadi: {error}"))?;
    let manifest = find_manifest(&current).ok_or_else(|| {
        "zet.toml bulunamadi. Komutu bir Zet projesi icinde calistirin.".to_string()
    })?;
    load_project(&manifest)
}

pub fn load_project(manifest_path: &Path) -> Result<Project, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|error| format!("zet.toml okunamadi ({}): {error}", manifest_path.display()))?;
    let manifest = parse_manifest(&content)?;
    let root = manifest_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    Ok(Project {
        name: manifest.name,
        version: manifest.version,
        source: root.join(manifest.entry),
        root,
        manifest_path: Some(manifest_path.to_path_buf()),
        dependencies: manifest.dependencies,
    })
}

pub fn parse_manifest(content: &str) -> Result<ManifestData, String> {
    let mut section = String::new();
    let mut package_values = HashMap::new();
    let mut dependencies = Vec::new();

    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        match section.as_str() {
            "package" => {
                let (key, raw_value) = line
                    .split_once('=')
                    .ok_or_else(|| format!("zet.toml:{} gecersiz paket satiri.", index + 1))?;
                let value = parse_string_value(raw_value.trim()).ok_or_else(|| {
                    format!(
                        "zet.toml:{} degeri cift tirnak icinde olmalidir.",
                        index + 1
                    )
                })?;
                package_values.insert(key.trim().to_string(), value);
            }
            "dependencies" => {
                let (name, raw_value) = line
                    .split_once('=')
                    .ok_or_else(|| format!("zet.toml:{} gecersiz bagimlilik satiri.", index + 1))?;
                let name = name.trim().to_string();
                validate_dependency_name(&name)?;
                let values = parse_inline_table(raw_value.trim()).ok_or_else(|| {
                    format!(
                        "zet.toml:{} bagimliligi {{ git = \"...\", version = \"...\" }} biciminde olmalidir.",
                        index + 1
                    )
                })?;
                let git = values.get("git").cloned().ok_or_else(|| {
                    format!("zet.toml:{} bagimliliginda git alani eksik.", index + 1)
                })?;
                let version = values.get("version").cloned().ok_or_else(|| {
                    format!("zet.toml:{} bagimliliginda version alani eksik.", index + 1)
                })?;
                let registry = match values.get("registry").map(String::as_str) {
                    None => false,
                    Some("zet") => true,
                    Some(value) => {
                        return Err(format!(
                            "zet.toml:{} bilinmeyen registry degeri: {value}",
                            index + 1
                        ));
                    }
                };
                dependencies.push(ManifestDependency {
                    name,
                    git,
                    version,
                    registry,
                });
            }
            _ => {}
        }
    }

    let name = package_values
        .remove("name")
        .ok_or_else(|| "zet.toml icinde [package].name eksik.".to_string())?;
    validate_project_name(&name)?;
    dependencies.sort_by(|left, right| left.name.cmp(&right.name));
    let mut dependency_names = HashSet::new();
    for dependency in &dependencies {
        if !dependency_names.insert(&dependency.name) {
            return Err(format!(
                "zet.toml icinde yinelenen bagimlilik: {}",
                dependency.name
            ));
        }
    }

    let entry = PathBuf::from(
        package_values
            .remove("entry")
            .unwrap_or_else(|| DEFAULT_ENTRY.to_string()),
    );
    validate_entry_path(&entry)?;

    Ok(ManifestData {
        name,
        version: package_values
            .remove("version")
            .unwrap_or_else(|| "0.1.0".to_string()),
        description: package_values.remove("description").unwrap_or_default(),
        entry,
        dependencies,
    })
}

pub fn set_dependency(manifest_path: &Path, dependency: &ManifestDependency) -> Result<(), String> {
    validate_dependency_name(&dependency.name)?;
    let content = fs::read_to_string(manifest_path)
        .map_err(|error| format!("zet.toml okunamadi: {error}"))?;
    let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let replacement = dependency_line(dependency);

    if let Some((start, end)) = section_bounds(&lines, "dependencies") {
        if let Some(index) = (start + 1..end).find(|index| {
            lines[*index]
                .split_once('=')
                .map(|(name, _)| name.trim() == dependency.name)
                .unwrap_or(false)
        }) {
            lines[index] = replacement;
        } else {
            lines.insert(end, replacement);
        }
    } else {
        if lines
            .last()
            .map(|line| !line.trim().is_empty())
            .unwrap_or(false)
        {
            lines.push(String::new());
        }
        lines.push("[dependencies]".to_string());
        lines.push(replacement);
    }

    write_manifest_lines(manifest_path, &lines)
}

pub fn remove_dependency(manifest_path: &Path, name: &str) -> Result<bool, String> {
    validate_dependency_name(name)?;
    let content = fs::read_to_string(manifest_path)
        .map_err(|error| format!("zet.toml okunamadi: {error}"))?;
    let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let Some((start, end)) = section_bounds(&lines, "dependencies") else {
        return Ok(false);
    };
    let Some(index) = (start + 1..end).find(|index| {
        lines[*index]
            .split_once('=')
            .map(|(dependency_name, _)| dependency_name.trim() == name)
            .unwrap_or(false)
    }) else {
        return Ok(false);
    };
    lines.remove(index);
    write_manifest_lines(manifest_path, &lines)?;
    Ok(true)
}

pub fn prepare_build_layout(
    runtime_template: &Path,
    project_root: &Path,
) -> Result<BuildLayout, String> {
    let template_manifest = runtime_template.join("Cargo.toml");
    if !template_manifest.is_file() {
        return Err(format!(
            "Zet runtime bulunamadi: {}\nZET_RUNTIME_DIR degiskenini runtime klasorune yonlendirin.",
            template_manifest.display()
        ));
    }

    let zet_dir = project_root.join(".zet");
    let runtime_dir = zet_dir.join("runtime");
    let source_dir = runtime_dir.join("src");
    let target_dir = zet_dir.join("target");
    let bin_dir = zet_dir.join("bin");
    fs::create_dir_all(&source_dir)
        .and_then(|_| fs::create_dir_all(&target_dir))
        .and_then(|_| fs::create_dir_all(&bin_dir))
        .map_err(|error| format!(".zet calisma dizini olusturulamadi: {error}"))?;

    copy_if_changed(&template_manifest, &runtime_dir.join("Cargo.toml"))?;
    let template_lock = runtime_template.join("Cargo.lock");
    if template_lock.is_file() {
        copy_if_changed(&template_lock, &runtime_dir.join("Cargo.lock"))?;
    }

    Ok(BuildLayout {
        generated_source: source_dir.join("app.rs"),
        runtime_dir,
        target_dir,
        bin_dir,
    })
}

pub fn write_if_changed(path: &Path, content: &[u8]) -> Result<(), String> {
    if fs::read(path).ok().as_deref() == Some(content) {
        return Ok(());
    }
    fs::write(path, content)
        .map_err(|error| format!("Dosya yazilamadi ({}): {error}", path.display()))
}

pub fn validate_dependency_name(name: &str) -> Result<(), String> {
    let mut characters = name.chars();
    let valid_start = characters
        .next()
        .map(|character| character.is_ascii_alphabetic() || character == '_')
        .unwrap_or(false);
    let valid_rest =
        characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
    if valid_start && valid_rest {
        Ok(())
    } else {
        Err(format!(
            "Gecersiz paket adi '{name}'. Import uyumlulugu icin harf veya '_' ile baslayip yalnizca harf, rakam ve '_' kullanmalidir."
        ))
    }
}

fn validate_entry_path(entry: &Path) -> Result<(), String> {
    let unsafe_component = entry.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });
    if entry.as_os_str().is_empty() || entry.is_absolute() || unsafe_component {
        return Err(format!(
            "Gecersiz paket girisi '{}'. entry proje dizini icinde goreli bir yol olmalidir.",
            entry.display()
        ));
    }
    Ok(())
}

pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(directory) = current {
        let candidate = directory.join("zet.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = directory.parent();
    }
    None
}

fn parse_string_value(value: &str) -> Option<String> {
    let value = value.trim();
    if !value.starts_with('"') {
        return None;
    }
    let rest = &value[1..];
    let closing = rest.find('"')?;
    let trailing = rest[closing + 1..].trim();
    if !trailing.is_empty() && !trailing.starts_with('#') {
        return None;
    }
    Some(rest[..closing].to_string())
}

fn parse_inline_table(value: &str) -> Option<HashMap<String, String>> {
    let inner = value.strip_prefix('{')?.strip_suffix('}')?;
    let mut values = HashMap::new();
    for field in inner.split(',') {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, raw_value) = field.split_once('=')?;
        values.insert(
            key.trim().to_string(),
            parse_string_value(raw_value.trim())?,
        );
    }
    Some(values)
}

fn section_bounds(lines: &[String], section_name: &str) -> Option<(usize, usize)> {
    let header = format!("[{section_name}]");
    let start = lines.iter().position(|line| line.trim() == header)?;
    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            let line = line.trim();
            line.starts_with('[') && line.ends_with(']')
        })
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());
    Some((start, end))
}

fn dependency_line(dependency: &ManifestDependency) -> String {
    if dependency.registry {
        format!(
            "{} = {{ git = \"{}\", version = \"{}\", registry = \"zet\" }}",
            dependency.name, dependency.git, dependency.version
        )
    } else {
        format!(
            "{} = {{ git = \"{}\", version = \"{}\" }}",
            dependency.name, dependency.git, dependency.version
        )
    }
}

fn write_manifest_lines(manifest_path: &Path, lines: &[String]) -> Result<(), String> {
    let mut content = lines.join("\n");
    content.push('\n');
    fs::write(manifest_path, content).map_err(|error| format!("zet.toml yazilamadi: {error}"))
}

fn validate_project_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        });
    if valid {
        Ok(())
    } else {
        Err(format!(
            "Gecersiz proje adi '{name}'. Yalnizca harf, rakam, '-' ve '_' kullanin."
        ))
    }
}

fn absolutize(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    env::current_dir()
        .map(|current| current.join(path))
        .map_err(|error| format!("Calisma dizini okunamadi: {error}"))
}

fn copy_if_changed(source: &Path, destination: &Path) -> Result<(), String> {
    let content = fs::read(source)
        .map_err(|error| format!("Runtime dosyasi okunamadi ({}): {error}", source.display()))?;
    write_if_changed(destination, &content)
}
