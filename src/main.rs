mod ast;
mod parser;
mod codegen;
mod analysis;

use std::env;
use std::fs;
use std::process::Command;
use std::collections::HashMap;
use crate::analysis::determinism::{DeterminismAnalyzer, SymbolTable};
use crate::analysis::taint::TaintAnalyzer;
use crate::analysis::scope::ScopeAnalyzer;

fn main() {

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Kullanim: gojo <dosya.gojo>");
        return;
    }

    let filename = &args[1];

    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            println!("Dosya okunamadi!");
            return;
        }
    };

    // 1. PARSER (ARTIK parse_program ÇAĞIRIYORUZ)
    let (_, functions) = match parser::parse_program(&content) {
        Ok(res) => res,
        Err(e) => {
            println!("Syntax Hatası:\n{:?}", e);
            return;
        }
    };
    println!("Parser: {} fonksiyon bulundu.", functions.len());

    // Sembol tablosunu hazırla (Tüm fonksiyonları kaydet)
    let mut func_map = HashMap::new();
    for f in &functions {
        func_map.insert(f.name.clone(), f.clone());
    }
    let symbols = SymbolTable { functions: func_map };

    // 2. GÜVENLİK (Tüm fonksiyonları tek tek tara)
    for func in &functions {
        if let Err(e) = DeterminismAnalyzer::check(func, &symbols) { println!("DETERMINISM HATASI ({}): {}", func.name, e); return; }
        if let Err(e) = TaintAnalyzer::check(func) { println!("TAINT HATASI ({}): {}", func.name, e); return; }
        let mut scope_pass = ScopeAnalyzer::new();
        if let Err(e) = scope_pass.analyze(func) { println!("SCOPE HATASI ({}): {}", func.name, e); return; }
    }

    // 3. CODEGEN (Listeyi gönder)
    let mut generator = codegen::Codegen::new();
    let rust_code = generator.generate(&functions);

    let output_path = "src/app.rs";
    if let Err(_) = fs::write(output_path, rust_code) {
         println!("Rust dosyasi yazilamadi.");
         return;
    }

    println!("Derleniyor ve Çalıştırılıyor...");
    
    let status = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--quiet")
        .arg("--bin") 
        .arg("app")   
        .status();

    match status {
        Ok(s) if s.success() => println!(""),
        _ => println!("Çalışma zamanı hatası!"),
    }
}