use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_ENTRY: &str = "src/main.zt";

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub version: String,
    pub root: PathBuf,
    pub source: PathBuf,
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

    let manifest =
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nentry = \"{DEFAULT_ENTRY}\"\n");
    let main_source =
        format!("nondet fn main() -> Void {{\n    println(\"Merhaba, {name}!\")\n}}\n");

    fs::write(target.join("zet.toml"), manifest)
        .map_err(|error| format!("zet.toml yazilamadi: {error}"))?;
    fs::write(source_dir.join("main.zt"), main_source)
        .map_err(|error| format!("Ana kaynak dosyasi yazilamadi: {error}"))?;
    fs::write(target.join(".gitignore"), "/.zet/\n")
        .map_err(|error| format!(".gitignore yazilamadi: {error}"))?;

    Ok(Project {
        name,
        version: "0.1.0".to_string(),
        root: target.clone(),
        source: target.join(DEFAULT_ENTRY),
    })
}

pub fn resolve_project(explicit_source: Option<&Path>) -> Result<Project, String> {
    if let Some(source) = explicit_source {
        let source = absolutize(source)?;
        if !source.is_file() {
            return Err(format!("Kaynak dosyasi bulunamadi: {}", source.display()));
        }

        if let Some(manifest) = find_manifest(source.parent().unwrap_or(Path::new("."))) {
            let mut project = load_manifest(&manifest)?;
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
        });
    }

    let current =
        env::current_dir().map_err(|error| format!("Calisma dizini okunamadi: {error}"))?;
    let manifest = find_manifest(&current).ok_or_else(|| {
        "zet.toml bulunamadi. Bir proje icinde calistirin veya bir .zt dosyasi belirtin."
            .to_string()
    })?;
    let project = load_manifest(&manifest)?;
    if !project.source.is_file() {
        return Err(format!(
            "Proje giris dosyasi bulunamadi: {}",
            project.source.display()
        ));
    }
    Ok(project)
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

fn load_manifest(manifest_path: &Path) -> Result<Project, String> {
    let content = fs::read_to_string(manifest_path)
        .map_err(|error| format!("zet.toml okunamadi ({}): {error}", manifest_path.display()))?;
    let mut section = String::new();
    let mut name = None;
    let mut version = None;
    let mut entry = None;

    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        if section != "package" {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("zet.toml:{} gecersiz satir.", index + 1));
        };
        let value = parse_string_value(value.trim()).ok_or_else(|| {
            format!(
                "zet.toml:{} degeri cift tirnak icinde olmalidir.",
                index + 1
            )
        })?;
        match key.trim() {
            "name" => name = Some(value),
            "version" => version = Some(value),
            "entry" => entry = Some(value),
            _ => {}
        }
    }

    let name = name.ok_or_else(|| "zet.toml icinde [package].name eksik.".to_string())?;
    validate_project_name(&name)?;
    let version = version.unwrap_or_else(|| "0.1.0".to_string());
    let root = manifest_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let source = root.join(entry.unwrap_or_else(|| DEFAULT_ENTRY.to_string()));

    Ok(Project {
        name,
        version,
        root,
        source,
    })
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

fn find_manifest(start: &Path) -> Option<PathBuf> {
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
