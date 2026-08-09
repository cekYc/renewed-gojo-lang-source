mod ast;
mod parser;
mod codegen;
mod analysis;
mod lsp;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::collections::HashMap;
use crate::ast::{TopLevel, FunctionDef, StructDef};
use crate::analysis::determinism::{DeterminismAnalyzer, SymbolTable};
use crate::analysis::taint::TaintAnalyzer;
use crate::analysis::scope::ScopeAnalyzer;

fn resolve_imports(base_path: &Path, toplevels: Vec<TopLevel>, loaded: &mut HashMap<PathBuf, Vec<TopLevel>>) -> Result<Vec<TopLevel>, String> {
    let mut resolved = Vec::new();
    for item in toplevels {
        if let TopLevel::Import(path) = item {
            let mut file_path = base_path.to_path_buf();
            for p in &path { file_path.push(p); }
            file_path.set_extension("zt");
            
            let canonical = file_path.canonicalize().unwrap_or(file_path.clone());
            if loaded.contains_key(&canonical) {
                // Already loaded, just emit the import statement to establish the module structure if needed
                resolved.push(TopLevel::Import(path));
                continue;
            }
            
            let content = match fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(e) => return Err(format!("Modül okunamadı: {:?} - {}", file_path, e)),
            };
            let content = content.trim_start_matches('\u{feff}');
            
            let (_, items) = match parser::parse_program(&content) {
                Ok(res) => res,
                Err(e) => return Err(format!("Syntax Hatası ({:?}):\n{:?}", file_path, e)),
            };
            
            loaded.insert(canonical.clone(), vec![]); // prevent cycles
            
            let parent_dir = file_path.parent().unwrap_or(Path::new(""));
            let resolved_items = resolve_imports(parent_dir, items, loaded)?;
            
            loaded.insert(canonical, resolved_items.clone());
            
            // Build the nested TopLevel::Module structure for this import
            let module_name = path.last().unwrap().clone();
            let mut nested = TopLevel::Module(module_name, resolved_items);
            for i in (0..path.len()-1).rev() {
                nested = TopLevel::Module(path[i].clone(), vec![nested]);
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
            if let Some(existing) = merged.iter_mut().find(|m| if let TopLevel::Module(n, _) = m { n == &name } else { false }) {
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

// Flattens functions for analysis. For simplicity, we just extract all functions regardless of module scope
// In a real module system, we would qualify names.
pub fn extract_functions(items: &[TopLevel], funcs: &mut Vec<FunctionDef>, current_path: Vec<String>) {
    for item in items {
        match item {
            TopLevel::Function(f) => {
                let mut f_clone = f.clone();
                let mut path = current_path.clone();
                path.push(f.name.clone());
                f_clone.name = path.join("::");
                funcs.push(f_clone);
            }
            TopLevel::Module(name, inner) => {
                let mut new_path = current_path.clone();
                new_path.push(name.clone());
                extract_functions(inner, funcs, new_path);
            }
            _ => {}
        }
    }
}

fn print_usage() {
    println!("Zet compiler v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Kullanim:");
    println!("  zet <dosya.zt>        Zet programini derle ve calistir");
    println!("  zet --lsp             Dil sunucusunu baslat");
    println!("  zet --version         Surum bilgisini goster");
    println!("  zet --help            Bu yardimi goster");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("zet-compiler {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return;
    }

    if args.iter().any(|a| a == "--lsp") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            lsp::run_lsp().await;
        });
        return;
    }

    if args.len() < 2 {
        print_usage();
        exit(2);
    }

    let filename = &args[1];
    let base_path = Path::new(filename).parent().unwrap_or(Path::new(""));

    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(e) => { eprintln!("Dosya okunamadi: {}", e); exit(1); }
    };
    let content = content.trim_start_matches('\u{feff}');

    let (remaining, toplevels) = match parser::parse_program(&content) {
        Ok(res) => res,
        Err(e) => { eprintln!("Syntax Hatası:\n{:?}", e); exit(1); }
    };
    if !remaining.trim().is_empty() {
        eprintln!("PARSER STOPPED EARLY. Remaining:\n{}", remaining.trim().chars().take(200).collect::<String>());
        exit(1);
    }
    
    let mut loaded = HashMap::new();
    let resolved_toplevels = match resolve_imports(base_path, toplevels, &mut loaded) {
        Ok(r) => r,
        Err(e) => { eprintln!("{}", e); exit(1); }
    };
    
    // We group modules by name in codegen to prevent duplicate `pub mod a` declarations.
    // For analysis, we just extract all functions.
    let mut all_functions = Vec::new();
    extract_functions(&resolved_toplevels, &mut all_functions, Vec::new());
    
    println!("[Zet Parser] {} ana bileşen, toplam {} fonksiyon bulundu.", resolved_toplevels.len(), all_functions.len());

    let mut func_map = HashMap::new();
    for f in &all_functions {
        func_map.insert(f.name.clone(), f.clone());
    }
    let symbols = SymbolTable { functions: func_map };

    for func in &all_functions {
        if let Err(e) = DeterminismAnalyzer::check(func, &symbols) { eprintln!("[ZET HATA] Determinizm ({}): {}", func.name, e); exit(1); }
        if let Err(e) = TaintAnalyzer::check(func, &symbols) { eprintln!("[ZET HATA] Taint ({}): {}", func.name, e); exit(1); }
        let mut scope_pass = ScopeAnalyzer::new();
        if let Err(e) = scope_pass.analyze(func) { eprintln!("[ZET HATA] Scope ({}): {}", func.name, e); exit(1); }
    }

    let mut generator = codegen::Codegen::new();
    let rust_code = generator.generate(&resolved_toplevels);

    let output_path = "src/app.rs";
    if let Err(_) = fs::write(output_path, rust_code) {
         eprintln!("Rust dosyasi yazilamadi.");
         exit(1);
    }

    println!("[Zet v{}] Derleniyor ve Çalıştırılıyor...", env!("CARGO_PKG_VERSION"));
    
    let user_args: Vec<String> = args[2..].to_vec();
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("--release").arg("--quiet").arg("--bin").arg("app").arg("--");
    for arg in &user_args { cmd.arg(arg); }
    
    let status = cmd.status();
    match status {
        Ok(s) if s.success() => println!(""),
        Ok(s) => {
            eprintln!("Çalışma zamanı hatası!");
            exit(s.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Cargo başlatılamadı: {}", e);
            exit(1);
        }
    }
}
