mod analysis;
mod ast;
mod codegen;
mod lsp;
mod package;
mod parser;
mod project;
mod registry;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use crate::analysis::determinism::{DeterminismAnalyzer, SymbolTable};
use crate::analysis::scope::ScopeAnalyzer;
use crate::analysis::taint::TaintAnalyzer;
use crate::ast::{FunctionDef, TopLevel};

fn resolve_imports(
    base_path: &Path,
    toplevels: Vec<TopLevel>,
    loaded: &mut HashMap<PathBuf, Vec<TopLevel>>,
    package_imports: &HashMap<String, package::PackageImport>,
) -> Result<Vec<TopLevel>, String> {
    let mut resolved = Vec::new();
    for item in toplevels {
        if let TopLevel::Import(path) = item {
            let mut file_path = base_path.to_path_buf();
            for part in &path {
                file_path.push(part);
            }
            file_path.set_extension("zt");

            if !file_path.is_file() {
                if let Some(package) = path.first().and_then(|name| package_imports.get(name)) {
                    file_path = if path.len() == 1 {
                        package.entry.clone()
                    } else {
                        let mut package_file = package.source_root.clone();
                        for part in &path[1..] {
                            package_file.push(part);
                        }
                        package_file.set_extension("zt");
                        package_file
                    };
                }
            }

            let canonical = file_path.canonicalize().unwrap_or(file_path.clone());
            if loaded.contains_key(&canonical) {
                resolved.push(TopLevel::Import(path));
                continue;
            }

            let content = fs::read_to_string(&file_path)
                .map_err(|error| format!("Modul okunamadi: {:?} - {error}", file_path))?;
            let content = content.trim_start_matches('\u{feff}');
            let (_, items) = parser::parse_program(content)
                .map_err(|error| format!("Syntax hatasi ({:?}):\n{:?}", file_path, error))?;

            loaded.insert(canonical.clone(), Vec::new());
            let parent_dir = file_path.parent().unwrap_or(Path::new(""));
            let resolved_items = resolve_imports(parent_dir, items, loaded, package_imports)?;
            loaded.insert(canonical, resolved_items.clone());

            let module_name = path
                .last()
                .ok_or_else(|| "Bos modul yolu kullanilamaz.".to_string())?
                .clone();
            let mut nested = TopLevel::Module(module_name, resolved_items);
            for index in (0..path.len() - 1).rev() {
                nested = TopLevel::Module(path[index].clone(), vec![nested]);
            }
            resolved.push(nested);
        } else {
            resolved.push(item);
        }
    }
    Ok(merge_toplevels(resolved))
}

fn merge_toplevels(items: Vec<TopLevel>) -> Vec<TopLevel> {
    let mut merged: Vec<TopLevel> = Vec::new();
    for item in items {
        if let TopLevel::Module(name, inner) = item {
            if let Some(existing) = merged.iter_mut().find(|module| {
                if let TopLevel::Module(existing_name, _) = module {
                    existing_name == &name
                } else {
                    false
                }
            }) {
                if let TopLevel::Module(_, existing_inner) = existing {
                    existing_inner.extend(inner);
                    let new_inner = merge_toplevels(std::mem::take(existing_inner));
                    *existing_inner = new_inner;
                }
            } else {
                merged.push(TopLevel::Module(name, merge_toplevels(inner)));
            }
        } else {
            merged.push(item);
        }
    }
    merged
}

pub fn extract_functions(
    items: &[TopLevel],
    functions: &mut Vec<FunctionDef>,
    current_path: Vec<String>,
) {
    for item in items {
        match item {
            TopLevel::Function(function) => {
                let mut function = function.clone();
                let mut path = current_path.clone();
                path.push(function.name.clone());
                function.name = path.join("::");
                functions.push(function);
            }
            TopLevel::Module(name, inner) => {
                let mut path = current_path.clone();
                path.push(name.clone());
                extract_functions(inner, functions, path);
            }
            _ => {}
        }
    }
}

fn print_usage() {
    println!("Zet compiler v{}", env!("CARGO_PKG_VERSION"));
    println!("Platform: {}-{}", env::consts::OS, env::consts::ARCH);
    println!();
    println!("Kullanim:");
    println!("  zet new <ad>              Yeni bir Zet projesi olustur");
    println!("  zet run [dosya.zt]        Projeyi veya dosyayi calistir");
    println!("  zet build [dosya.zt]      Yerel calistirilabilir dosya uret");
    println!("  zet add <paket|depo>       Registry veya Git paketini ekle");
    println!("  zet remove <paket>        Bagimliligi kaldir");
    println!("  zet install               zet.lock paketlerini kur");
    println!("  zet update [paket]        Paketleri guncelle");
    println!("  zet search [sorgu]        Merkezi kayitta paket ara");
    println!("  zet publish [--dry-run]   Paketi dogrula ve kayda gonder");
    println!("  zet <dosya.zt> [arg...]   Tek dosyayi derle ve calistir");
    println!("  zet --lsp                 Dil sunucusunu baslat");
    println!("  zet --version             Surum bilgisini goster");
    println!("  zet --help                Bu yardimi goster");
    println!();
    println!("Program argumanlari icin: zet run -- <argumanlar>");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if matches!(args.get(1).map(String::as_str), Some("--version" | "-V")) {
        println!("zet-compiler {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if matches!(args.get(1).map(String::as_str), Some("--help" | "-h")) {
        print_usage();
        return;
    }
    if matches!(args.get(1).map(String::as_str), Some("--lsp")) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            lsp::run_lsp().await;
        });
        return;
    }
    if args.len() < 2 {
        print_usage();
        exit(2);
    }

    match args[1].as_str() {
        "new" => create_project_command(&args[2..]),
        "run" => {
            let (source, user_args) = parse_run_args(&args[2..]);
            execute(source.as_deref(), BuildMode::Run, &user_args);
        }
        "build" => {
            let source = parse_build_args(&args[2..]);
            execute(source.as_deref(), BuildMode::Build, &[]);
        }
        "add" => package_command(&args[2..], PackageCommand::Add),
        "remove" => package_command(&args[2..], PackageCommand::Remove),
        "install" => package_command(&args[2..], PackageCommand::Install),
        "update" => package_command(&args[2..], PackageCommand::Update),
        "search" => search_command(&args[2..]),
        "publish" => publish_command(&args[2..]),
        _ => execute(Some(Path::new(&args[1])), BuildMode::Run, &args[2..]),
    }
}

fn search_command(args: &[String]) {
    if args.len() > 1 {
        eprintln!("Kullanim: zet search [sorgu]");
        exit(2);
    }
    if let Err(error) = registry::search(args.first().map(String::as_str).unwrap_or("")) {
        eprintln!("[ZET KAYIT HATASI] {error}");
        exit(1);
    }
}

fn publish_command(args: &[String]) {
    let dry_run = match args {
        [] => false,
        [argument] if argument == "--dry-run" => true,
        _ => {
            eprintln!("Kullanim: zet publish [--dry-run]");
            exit(2);
        }
    };
    let project = project::resolve_manifest_project().unwrap_or_else(|error| {
        eprintln!("[ZET HATA] {error}");
        exit(1);
    });
    if let Err(error) = registry::publish(&project, dry_run) {
        eprintln!("[ZET YAYIN HATASI] {error}");
        exit(1);
    }
}

fn create_project_command(args: &[String]) {
    if args.len() != 1 {
        eprintln!("Kullanim: zet new <proje-adi>");
        exit(2);
    }
    match project::create_project(Path::new(&args[0])) {
        Ok(created) => {
            println!("Zet projesi olusturuldu: {}", created.root.display());
            println!("  cd \"{}\"", created.root.display());
            println!("  zet run");
        }
        Err(error) => {
            eprintln!("[ZET HATA] {error}");
            exit(1);
        }
    }
}

fn parse_run_args(args: &[String]) -> (Option<PathBuf>, Vec<String>) {
    let separator = args.iter().position(|argument| argument == "--");
    let command_args = separator.map(|index| &args[..index]).unwrap_or(args);
    if command_args.len() > 1 {
        eprintln!("Kullanim: zet run [dosya.zt] [-- argumanlar]");
        exit(2);
    }
    let source = command_args.first().map(PathBuf::from);
    let user_args = separator
        .map(|index| args[index + 1..].to_vec())
        .unwrap_or_default();
    (source, user_args)
}

fn parse_build_args(args: &[String]) -> Option<PathBuf> {
    if args.len() > 1 || matches!(args.first().map(String::as_str), Some("--")) {
        eprintln!("Kullanim: zet build [dosya.zt]");
        exit(2);
    }
    args.first().map(PathBuf::from)
}

#[derive(Clone, Copy)]
enum BuildMode {
    Run,
    Build,
}

#[derive(Clone, Copy)]
enum PackageCommand {
    Add,
    Remove,
    Install,
    Update,
}

fn package_command(args: &[String], command: PackageCommand) {
    let valid = match command {
        PackageCommand::Add | PackageCommand::Remove => args.len() == 1,
        PackageCommand::Install => args.is_empty(),
        PackageCommand::Update => args.len() <= 1,
    };
    if !valid {
        let usage = match command {
            PackageCommand::Add => "zet add <paket_adi|sahip/depo[@surum]>",
            PackageCommand::Remove => "zet remove <paket>",
            PackageCommand::Install => "zet install",
            PackageCommand::Update => "zet update [paket]",
        };
        eprintln!("Kullanim: {usage}");
        exit(2);
    }

    let project = project::resolve_manifest_project().unwrap_or_else(|error| {
        eprintln!("[ZET HATA] {error}");
        exit(1);
    });
    let result = match command {
        PackageCommand::Add => package::add(&project, &args[0]),
        PackageCommand::Remove => package::remove(&project, &args[0]),
        PackageCommand::Install => package::install(&project),
        PackageCommand::Update => package::update(&project, args.first().map(String::as_str)),
    };
    if let Err(error) = result {
        eprintln!("[ZET PAKET HATASI] {error}");
        exit(1);
    }
}

fn execute(source: Option<&Path>, mode: BuildMode, user_args: &[String]) {
    let project = project::resolve_project(source).unwrap_or_else(|error| {
        eprintln!("[ZET HATA] {error}");
        exit(1);
    });
    let package_imports = package::import_map(&project).unwrap_or_else(|error| {
        eprintln!("[ZET PAKET HATASI] {error}");
        exit(1);
    });
    let filename = &project.source;
    let base_path = filename.parent().unwrap_or(Path::new(""));

    let content = fs::read_to_string(filename).unwrap_or_else(|error| {
        eprintln!("Dosya okunamadi ({}): {error}", filename.display());
        exit(1);
    });
    let content = content.trim_start_matches('\u{feff}');

    let (remaining, toplevels) = parser::parse_program(content).unwrap_or_else(|error| {
        eprintln!("Syntax hatasi:\n{error:?}");
        exit(1);
    });
    if !remaining.trim().is_empty() {
        let remaining: String = remaining.trim().chars().take(200).collect();
        eprintln!("Parser erken durdu. Kalan kaynak:\n{remaining}");
        exit(1);
    }

    let mut loaded = HashMap::new();
    let resolved_toplevels = resolve_imports(base_path, toplevels, &mut loaded, &package_imports)
        .unwrap_or_else(|error| {
            eprintln!("{error}");
            exit(1);
        });

    let mut all_functions = Vec::new();
    extract_functions(&resolved_toplevels, &mut all_functions, Vec::new());
    println!(
        "[Zet Parser] {} ana bilesen, toplam {} fonksiyon bulundu.",
        resolved_toplevels.len(),
        all_functions.len()
    );

    let mut function_map = HashMap::new();
    for function in &all_functions {
        function_map.insert(function.name.clone(), function.clone());
    }
    let symbols = SymbolTable {
        functions: function_map,
    };

    for function in &all_functions {
        if let Err(error) = DeterminismAnalyzer::check(function, &symbols) {
            eprintln!("[ZET HATA] Determinizm ({}): {error}", function.name);
            exit(1);
        }
        if let Err(error) = TaintAnalyzer::check(function, &symbols) {
            eprintln!("[ZET HATA] Taint ({}): {error}", function.name);
            exit(1);
        }
        let mut scope_pass = ScopeAnalyzer::new();
        if let Err(error) = scope_pass.analyze(function) {
            eprintln!("[ZET HATA] Scope ({}): {error}", function.name);
            exit(1);
        }
    }

    let mut generator = codegen::Codegen::new();
    let rust_code = generator.generate(&resolved_toplevels);
    let runtime_template = env::var_os("ZET_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let layout =
        project::prepare_build_layout(&runtime_template, &project.root).unwrap_or_else(|error| {
            eprintln!("[ZET HATA] {error}");
            exit(1);
        });
    project::write_if_changed(&layout.generated_source, rust_code.as_bytes()).unwrap_or_else(
        |error| {
            eprintln!("[ZET HATA] {error}");
            exit(1);
        },
    );

    let mut command = Command::new("cargo");
    command.current_dir(&layout.runtime_dir);
    command.env("CARGO_TARGET_DIR", &layout.target_dir);
    match mode {
        BuildMode::Run => {
            println!(
                "[Zet v{}] {} v{} derleniyor ve calistiriliyor...",
                env!("CARGO_PKG_VERSION"),
                project.name,
                project.version
            );
            command
                .arg("run")
                .arg("--release")
                .arg("--quiet")
                .arg("--bin")
                .arg("app")
                .arg("--");
            for argument in user_args {
                command.arg(argument);
            }
        }
        BuildMode::Build => {
            println!(
                "[Zet v{}] {} v{} derleniyor...",
                env!("CARGO_PKG_VERSION"),
                project.name,
                project.version
            );
            command
                .arg("build")
                .arg("--release")
                .arg("--quiet")
                .arg("--bin")
                .arg("app");
        }
    }

    match command.status() {
        Ok(status) if status.success() => {
            if matches!(mode, BuildMode::Build) {
                publish_binary(&project, &layout);
            } else {
                println!();
            }
        }
        Ok(status) => {
            eprintln!("Derleme veya calisma zamani hatasi!");
            exit(status.code().unwrap_or(1));
        }
        Err(error) => {
            eprintln!("Cargo baslatilamadi: {error}");
            exit(1);
        }
    }
}

fn publish_binary(project: &project::Project, layout: &project::BuildLayout) {
    let executable_name = if cfg!(windows) { "app.exe" } else { "app" };
    let built = layout.target_dir.join("release").join(executable_name);
    let output_name = if cfg!(windows) {
        format!("{}.exe", project.name)
    } else {
        project.name.clone()
    };
    let destination = layout.bin_dir.join(output_name);
    fs::copy(&built, &destination).unwrap_or_else(|error| {
        eprintln!(
            "[ZET HATA] Cikti kopyalanamadi ({} -> {}): {error}",
            built.display(),
            destination.display()
        );
        exit(1);
    });
    println!("Cikti: {}", destination.display());
}
