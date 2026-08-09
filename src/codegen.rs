use crate::ast::*;
use std::collections::HashSet;

pub struct Codegen { 
    indent_level: usize,
    pure_functions: HashSet<String>,
    is_current_func_pure: bool,
    is_current_func_result: bool,
    /// Scope bloğu içinde miyiz? (spawn → _zet_handles.push)
    in_scope: bool,
    /// For döngüsü içinde miyiz? (continue → break '_zet_body_N)
    in_for_loop: bool,
    /// Nested for-loop label counter
    for_label_id: usize,
    /// Current for-loop label id stack
    for_label_stack: Vec<usize>,
    routes: Vec<(String, String, String, usize, bool)>,
}

impl Codegen {
    pub fn new() -> Self { 
        Self { 
            indent_level: 0,
            pure_functions: HashSet::new(),
            is_current_func_pure: false,
            is_current_func_result: false,
            in_scope: false,
            in_for_loop: false,
            for_label_id: 0,
            for_label_stack: Vec::new(),
            routes: Vec::new(),
        } 
    }

    fn indent(&self) -> String { "    ".repeat(self.indent_level) }

    fn get_runtime_preamble(&self) -> String {
        r#"
#![allow(dead_code, unused_imports, unused_variables, unused_parens, unused_mut, non_snake_case)]
use ::std::time::Duration;
use ::std::io::{self, Write};
use serde_json::Value;

const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";   
const GREEN: &str = "\x1b[32m";  
const MAGENTA: &str = "\x1b[35m";
const YELLOW: &str = "\x1b[33m"; 
const BLUE: &str = "\x1b[34m";
const RED: &str = "\x1b[31m";

/// Zet v0.4 — Untrusted: Dış dünyadan gelen lekeli veri sarmalayıcısı.
#[derive(Clone, Debug)]
struct Untrusted(String);

impl Untrusted {
    fn validate(self) -> Result<String, String> {
        let s = self.0.trim().to_string();
        if s.is_empty() {
            Err("Dogrulama basarisiz: bos girdi.".to_string())
        } else {
            Ok(s)
        }
    }
}

struct Console;
impl Console {
    async fn read(prompt: String) -> Untrusted {
        print!("  {}[Console] {}: {}", BLUE, prompt, RESET);
        io::stdout().flush().unwrap();
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        Untrusted(buffer.trim().to_string())
    }
}

struct DB;
impl DB {
    async fn log(msg: String) {
        println!("  {}[LOG] {}{}", GREEN, msg, RESET);
    }
}

struct Util;
impl Util {
    #[inline(always)]
    async fn to_int(s: String) -> i64 { s.trim().parse::<i64>().unwrap_or(0) }
    #[inline(always)]
    async fn to_float(s: String) -> f64 { s.trim().parse::<f64>().unwrap_or(0.0) }
    #[inline(always)]
    async fn now() -> i64 {
        ::std::time::SystemTime::now()
            .duration_since(::std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }
}

struct HTTP;
impl HTTP {
    async fn get(url: String) -> Untrusted {
        let client = reqwest::Client::builder().user_agent("ZetLang/0.4").build().unwrap();
        match client.get(&url).send().await {
            Ok(res) => Untrusted(res.text().await.unwrap_or_else(|e| format!("Error: {}", e))),
            Err(e) => Untrusted(format!("Error: {}", e))
        }
    }
}

async fn input(prompt: String) -> Untrusted {
    print!("  {}[>] {}{}", CYAN, prompt, RESET);
    io::stdout().flush().unwrap();
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer);
    Untrusted(buffer.trim().to_string())
}

async fn inputln(prompt: String) -> Untrusted {
    println!("  {}[>] {}{}", CYAN, prompt, RESET);
    io::stdout().flush().unwrap();
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer);
    Untrusted(buffer.trim().to_string())
}

// ─── ZetAdd Trait (Polimorfik Toplama) ──────────────────────────
trait ZetAdd<Rhs> { type Output; fn z_add(self, rhs: Rhs) -> Self::Output; }
impl ZetAdd<i64> for i64 { type Output = i64; #[inline(always)] fn z_add(self, rhs: i64) -> i64 { self + rhs } }
impl ZetAdd<f64> for f64 { type Output = f64; #[inline(always)] fn z_add(self, rhs: f64) -> f64 { self + rhs } }
impl ZetAdd<String> for String { type Output = String; #[inline(always)] fn z_add(self, rhs: String) -> String { self + &rhs } }
impl<'a> ZetAdd<&'a str> for String { type Output = String; #[inline(always)] fn z_add(self, rhs: &'a str) -> String { self + rhs } }
impl ZetAdd<i64> for String { type Output = String; #[inline(always)] fn z_add(self, rhs: i64) -> String { format!("{}{}", self, rhs) } }
impl ZetAdd<f64> for String { type Output = String; #[inline(always)] fn z_add(self, rhs: f64) -> String { format!("{}{}", self, rhs) } }
impl ZetAdd<bool> for String { type Output = String; #[inline(always)] fn z_add(self, rhs: bool) -> String { format!("{}{}", self, rhs) } }
impl ZetAdd<char> for String { type Output = String; #[inline(always)] fn z_add(self, rhs: char) -> String { format!("{}{}", self, rhs) } }

// ─── ZetMul Trait (Polimorfik Çarpma) ───────────────────────────
trait ZetMul<Rhs> { type Output; fn z_mul(self, rhs: Rhs) -> Self::Output; }
impl ZetMul<i64> for i64 { type Output = i64; #[inline(always)] fn z_mul(self, rhs: i64) -> i64 { self * rhs } }
impl ZetMul<f64> for f64 { type Output = f64; #[inline(always)] fn z_mul(self, rhs: f64) -> f64 { self * rhs } }
impl ZetMul<i64> for String { type Output = String; fn z_mul(self, rhs: i64) -> String { self.repeat(rhs as usize) } }
impl<'a> ZetMul<i64> for &'a str { type Output = String; fn z_mul(self, rhs: i64) -> String { self.repeat(rhs as usize) } }
"#.to_string()
    }

    fn collect_metadata(&mut self, items: &Vec<TopLevel>, module_path: Vec<String>) {
        for item in items {
            match item {
                TopLevel::Function(func) => {
                    let mut full_path = module_path.clone();
                    // main function is generated as user_main in root module
                    if func.name == "main" && module_path.is_empty() {
                        full_path.push("user_main".to_string());
                    } else {
                        full_path.push(func.name.clone());
                    }
                    let rust_path = full_path.join("::");

                    if let Purity::Deterministic = func.purity {
                        self.pure_functions.insert(func.name.clone());
                    }
                    
                    for attr in &func.attributes {
                        let method = match attr.name.as_str() {
                            "get" | "post" | "put" | "delete" | "patch" => attr.name.clone(),
                            _ => continue,
                        };
                        if let Some(route_path) = attr.args.get(0) {
                            // Strip quotes from route_path
                            let clean_path = route_path.trim_matches('"').to_string();
                            let is_res = matches!(func.return_type, TypeRef::Result(_));
                            self.routes.push((method, clean_path, rust_path.clone(), func.params.len(), is_res));
                        }
                    }
                }
                TopLevel::Module(name, inner) => {
                    let mut new_path = module_path.clone();
                    new_path.push(name.clone());
                    self.collect_metadata(inner, new_path);
                }
                _ => {}
            }
        }
    }

    pub fn generate(&mut self, toplevels: &Vec<TopLevel>) -> String {
        self.pure_functions.clear();
        self.routes.clear();
        self.collect_metadata(toplevels, Vec::new());

        let mut code = self.get_runtime_preamble();
        
        let mut main_func = None;
        for item in toplevels {
            if let TopLevel::Function(f) = item {
                if f.name == "main" { main_func = Some(f.clone()); }
            }
            code.push_str(&self.generate_toplevel(item));
        }

        if let Some(main_fn) = main_func {
             code.push_str(&self.generate_main_shim(&main_fn));
        } else {
            eprintln!("\x1b[31m[Zet Hata]\x1b[0m 'main' fonksiyonu bulunamadi! Her Zet programinda bir 'main' fonksiyonu olmalidir.");
            eprintln!("\x1b[33mOrnek:\x1b[0m\n  nondet fn main() -> Void {{\n      println(\"Merhaba Dunya!\")\n  }}");
            std::process::exit(1);
        }
        code.push_str("\n\n");
        code.push_str(&self.generate_router());
        code
    }

    fn generate_toplevel(&mut self, item: &TopLevel) -> String {
        match item {
            TopLevel::Function(func) => self.generate_function(func),
            TopLevel::Struct(s) => {
                let mut code = format!("#[derive(Clone, Debug)]\npub struct {} {{\n", s.name);
                for field in &s.fields {
                    code.push_str(&format!("    pub {}: {},\n", field.name, self.map_type(&field.param_type)));
                }
                code.push_str("}\n\n");
                code
            }
            TopLevel::Module(name, inner) => {
                let mut code = format!("pub mod {} {{\n", name);
                code.push_str("    use super::*;\n");
                for child in inner {
                    // Indent all lines
                    let child_code = self.generate_toplevel(child);
                    for line in child_code.lines() {
                        code.push_str(&format!("    {}\n", line));
                    }
                }
                code.push_str("}\n\n");
                code
            }
            TopLevel::Import(_) => String::new(),
        }
    }

    fn generate_function(&mut self, func: &FunctionDef) -> String {
        let real_func_name = if func.name == "main" { "user_main" } else { &func.name };
        let params = func.params.iter().map(|p| format!("{}: {}", p.name, self.map_type(&p.param_type))).collect::<Vec<_>>().join(", ");
        
        let is_pure = self.pure_functions.contains(&func.name);
        self.is_current_func_pure = is_pure;
        self.is_current_func_result = matches!(func.return_type, TypeRef::Result(_));
        
        let async_keyword = if is_pure { "" } else { "async " };

        let mut code = format!("pub {}fn {}({}) -> {} {{\n", async_keyword, real_func_name, params, self.map_type(&func.return_type));
        self.indent_level += 1;
        code.push_str(&self.generate_block(&func.body));
        self.indent_level -= 1;
        code.push_str("}\n\n");
        code
    }

    fn generate_main_shim(&self, main_fn: &FunctionDef) -> String {
        let is_pure = self.pure_functions.contains(&main_fn.name);
        if main_fn.params.is_empty() {
            if is_pure {
                "fn main() {\n    user_main();\n}".to_string()
            } else {
                r#"#[tokio::main] async fn main() {
    user_main().await;
}"#.to_string()
            }
        } else {
            r#"#[tokio::main] async fn main() {
    let _zet_input = Untrusted(::std::env::args().skip(1).collect::<Vec<_>>().join(" "));
    user_main(_zet_input).await;
}"#.to_string()
        }
    }

    fn generate_router(&self) -> String {
        let mut code = "pub fn _zet_build_router() -> ::axum::Router {\n".to_string();
        code.push_str("    let mut app = ::axum::Router::new();\n");
        for (method, path, rust_func, params_len, is_res) in &self.routes {
            let axum_method = match method.as_str() {
                "get" => "get",
                "post" => "post",
                "put" => "put",
                "delete" => "delete",
                "patch" => "patch",
                _ => "get",
            };
            if *params_len == 1 {
                code.push_str(&format!("    app = app.route(\"{}\", ::axum::routing::{}(|body: ::axum::extract::Json<::serde_json::Value>| async move {{\n", path, axum_method));
                // Convert body to Untrusted JSON string
                code.push_str(&format!("        let payload = Untrusted(::serde_json::to_string(&body.0).unwrap_or_default());\n"));
                code.push_str(&format!("        let res = {}(payload).await;\n", rust_func));
            } else {
                code.push_str(&format!("    app = app.route(\"{}\", ::axum::routing::{}(|req: ::axum::extract::Request| async move {{\n", path, axum_method));
                code.push_str(&format!("        let res = {}().await;\n", rust_func));
            }
            if *is_res {
                code.push_str("        let res_str = match res { Ok(v) => v.to_string(), Err(e) => format!(\"Error: {}\", e) };\n");
                code.push_str("        ::axum::response::IntoResponse::into_response(res_str)\n");
            } else {
                code.push_str("        ::axum::response::IntoResponse::into_response(res.to_string())\n");
            }
            code.push_str("    }));\n");
        }
        code.push_str("        let cors = ::tower_http::cors::CorsLayer::permissive();\n");
        code.push_str("        app = app.layer(cors);\n");
        code.push_str("    app\n}\n");
        code
    }

    fn generate_block(&mut self, block: &Block) -> String {
        let mut code = String::new();
        for stmt in &block.statements { code.push_str(&self.generate_stmt(stmt)); }
        code
    }

    fn generate_stmt(&mut self, stmt: &Statement) -> String {
        let indent = self.indent();
        match stmt {
            Statement::Let(s) => format!("{}let mut {} = {};\n", indent, s.name, self.generate_expr(&s.value)),
            Statement::Const { name, value } => format!("{}let {} = {};\n", indent, name, self.generate_expr(value)),
            Statement::Assign { name, op, value } => format!("{}{} {} {};\n", indent, name, self.map_assign_op(op), self.generate_expr(value)),
            Statement::IndexAssign { name, index, op, value } => format!("{}{}[({} as usize)] {} {};\n", indent, name, self.generate_expr(index), self.map_assign_op(op), self.generate_expr(value)),
            Statement::Match { expr, arms } => {
                let has_string_pattern = arms.iter().any(|a| matches!(a.pattern, Pattern::Literal(Literal::Str(_))));
                let expr_str = self.generate_expr(expr);
                let match_target = if has_string_pattern { format!("{}.as_str()", expr_str) } else { expr_str };
                
                let mut s = format!("{}match {} {{\n", indent, match_target);
                self.indent_level += 1;
                for arm in arms {
                    let pat_str = match &arm.pattern {
                        Pattern::Wildcard => "_".to_string(),
                        Pattern::Identifier(n) => n.clone(),
                        Pattern::Literal(Literal::Str(st)) => {
                            let escaped = st.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t");
                            format!("\"{}\"", escaped)
                        },
                        Pattern::Literal(Literal::Int(i)) => i.to_string(),
                        Pattern::Literal(Literal::Float(f)) => {
                            let fs = format!("{}", f);
                            if fs.contains('.') { format!("{}f64", fs) } else { format!("{}.0f64", fs) }
                        },
                        Pattern::Literal(Literal::Bool(b)) => b.to_string(),
                        Pattern::Literal(Literal::Char(c)) => {
                            let escaped = match c {
                                '\n' => "\\n".to_string(), '\t' => "\\t".to_string(),
                                '\\' => "\\\\".to_string(), '\'' => "\\'".to_string(),
                                '\0' => "\\0".to_string(),
                                _ => c.to_string(),
                            };
                            format!("'{}'", escaped)
                        },
                    };
                    s.push_str(&format!("{}{} => {{\n", self.indent(), pat_str));
                    self.indent_level += 1;
                    s.push_str(&self.generate_block(&arm.body));
                    self.indent_level -= 1;
                    s.push_str(&format!("{}}},\n", self.indent()));
                }
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", indent));
                s
            }
            Statement::ExprStmt(e) => format!("{}{};\n", indent, self.generate_expr(e)),
            Statement::Break => {
                if self.in_for_loop {
                    let id = *self.for_label_stack.last().unwrap();
                    format!("{}break '_zet_for_{};\n", indent, id)
                } else {
                    format!("{}break;\n", indent)
                }
            }
            Statement::Continue => {
                if self.in_for_loop {
                    let id = *self.for_label_stack.last().unwrap();
                    format!("{}break '_zet_body_{};\n", indent, id)
                } else {
                    format!("{}continue;\n", indent)
                }
            }
            Statement::While { condition, body } => {
                let mut s = format!("{}while {} {{\n", indent, self.generate_expr(condition));
                self.indent_level += 1;
                s.push_str(&self.generate_block(body));
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", indent));
                s
            }
            Statement::For { var, start, end, step, body } => {
                let step_expr = if let Some(s) = step { self.generate_expr(s) } else { "1".to_string() };
                let label_id = self.for_label_id;
                self.for_label_id += 1;
                let mut s = format!("{}{{\n", indent); 
                self.indent_level += 1;
                s.push_str(&format!("{}let mut {} = {};\n", self.indent(), var, self.generate_expr(start)));
                s.push_str(&format!("{}let _zet_end = {};\n", self.indent(), self.generate_expr(end)));
                s.push_str(&format!("{}let _zet_step = {};\n", self.indent(), step_expr));
                s.push_str(&format!("{}'_zet_for_{}: while (_zet_step > 0 && {} < _zet_end) || (_zet_step < 0 && {} > _zet_end) {{\n", self.indent(), label_id, var, var));
                self.indent_level += 1;
                // Wrap body in labeled block so `continue` (→ break '_zet_body_N) skips to increment
                s.push_str(&format!("{}'_zet_body_{}: {{\n", self.indent(), label_id));
                self.indent_level += 1;
                let prev_in_for = self.in_for_loop;
                self.in_for_loop = true;
                self.for_label_stack.push(label_id);
                s.push_str(&self.generate_block(body));
                self.for_label_stack.pop();
                self.in_for_loop = prev_in_for;
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", self.indent()));
                
                if self.is_current_func_pure {
                     s.push_str(&format!("{}{} += _zet_step;\n", self.indent(), var));
                } else {
                     s.push_str(&format!("{}{} = {}.z_add(_zet_step);\n", self.indent(), var, var));
                }
                
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", self.indent()));
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", indent));
                s
            }
            Statement::If { condition, then_block, else_block } => {
                let mut s = format!("{}if {} {{\n", indent, self.generate_expr(condition));
                self.indent_level += 1;
                s.push_str(&self.generate_block(then_block));
                self.indent_level -= 1;
                s.push_str(&format!("{}}}", indent));
                if let Some(else_b) = else_block {
                    s.push_str(" else {\n");
                    self.indent_level += 1;
                    s.push_str(&self.generate_block(else_b));
                    self.indent_level -= 1;
                    s.push_str(&format!("{}}}", indent));
                }
                s.push_str("\n");
                s
            }
            Statement::ScopeBlock { name, body } => {
                let mut s = format!("{}// Scope: {}\n{}{{\n", indent, name, indent);
                self.indent_level += 1;
                let inner_indent = self.indent();
                s.push_str(&format!("{}let mut _zet_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();\n", inner_indent));
                
                let was_in_scope = self.in_scope;
                self.in_scope = true;
                s.push_str(&self.generate_block(body));
                self.in_scope = was_in_scope;
                
                s.push_str(&format!("{}for _h in _zet_handles {{ _h.await.ok(); }}\n", inner_indent));
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", indent));
                s
            }
            Statement::ValidateBlock { target, success_scope, on_fail, .. } => {
                let mut s = format!("{}match {}.validate() {{\n", indent, target);
                self.indent_level += 1;
                let inner = self.indent();
                s.push_str(&format!("{}Ok({}) => {{\n", inner, target));
                self.indent_level += 1;
                s.push_str(&self.generate_block(success_scope));
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", inner));
                s.push_str(&format!("{}Err(_zet_err) => {{\n", inner));
                self.indent_level += 1;
                if on_fail.statements.is_empty() {
                    s.push_str(&format!("{}eprintln!(\"  {{}}[VALIDATE FAIL] {{}}{{}}\", RED, _zet_err, RESET);\n", self.indent()));
                } else {
                    s.push_str(&self.generate_block(on_fail));
                }
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", inner));
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", indent));
                s
            }
            Statement::Return(Some(e)) => {
                if self.is_current_func_result {
                    if let Expr::Call(name, args, _) = e {
                        if name == "error" && !args.is_empty() {
                            return format!("{}return Err({}.to_string());\n", indent, self.generate_expr(&args[0]));
                        }
                    }
                    format!("{}return Ok({});\n", indent, self.generate_expr(e))
                } else {
                    format!("{}return {};\n", indent, self.generate_expr(e))
                }
            }
            Statement::Return(None) => {
                if self.is_current_func_result {
                    format!("{}return Ok(());\n", indent)
                } else {
                    format!("{}return;\n", indent)
                }
            }
        }
    }

    fn generate_expr_as_string(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(Literal::Str(_)) => self.generate_expr(expr),
            Expr::Interpolation(_) => self.generate_expr(expr),
            _ => format!("format!(\"{{}}\", {})", self.generate_expr(expr))
        }
    }

    fn generate_expr_owned(&self, expr: &Expr) -> String {
        match expr {
            Expr::Identifier(s) => format!("{}.clone()", s),
            _ => self.generate_expr(expr)
        }
    }

    fn generate_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Identifier(s) => s.clone(),
            Expr::Literal(l) => match l {
                Literal::Int(i) => i.to_string(),
                Literal::Float(f) => {
                    let s = format!("{}", f);
                    if s.contains('.') { format!("{}f64", s) } else { format!("{}.0f64", s) }
                },
                Literal::Str(s) => {
                    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t");
                    format!("\"{}\".to_string()", escaped)
                },
                Literal::Bool(b) => b.to_string(),
                Literal::Char(c) => {
                    let escaped = match c {
                        '\n' => "\\n".to_string(), '\t' => "\\t".to_string(),
                        '\\' => "\\\\".to_string(), '\'' => "\\'".to_string(),
                        '\0' => "\\0".to_string(),
                        _ => c.to_string(),
                    };
                    format!("'{}'", escaped)
                },
            },
            Expr::Interpolation(parts) => {
                let mut fmt_str = String::new();
                let mut args: Vec<String> = Vec::new();
                for part in parts {
                    match part {
                        InterpolPart::Lit(s) => {
                            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t").replace('{', "{{").replace('}', "}}");
                            fmt_str.push_str(&escaped);
                        },
                        InterpolPart::Expr(e) => {
                            fmt_str.push_str("{}");
                            args.push(self.generate_expr(e));
                        },
                    }
                }
                if args.is_empty() {
                    format!("\"{}\".to_string()", fmt_str)
                } else {
                    format!("format!(\"{}\", {})", fmt_str, args.join(", "))
                }
            },
            Expr::Unary(op, inner) => {
                match op {
                    UnaryOp::Not => format!("(!{})", self.generate_expr(inner)),
                    UnaryOp::Neg => format!("(-{})", self.generate_expr(inner)),
                }
            },
            Expr::Infra(call) => {
                format!("tokio::time::timeout(Duration::from_millis({}), {}::{}({})).await.unwrap()", call.config.timeout_ms, call.service, call.method, call.args.iter().map(|a| self.generate_expr_as_string(a)).collect::<Vec<_>>().join(", "))
            },
            Expr::JsonField(source, key) => {
                format!(
                    "serde_json::from_str::<serde_json::Value>(&{}).ok().and_then(|v| v.get(\"{}\").map(|x| if x.is_string() {{ x.as_str().unwrap().to_string() }} else {{ x.to_string() }})).unwrap_or(\"HATA\".to_string())", 
                    self.generate_expr(source), key
                )
            },
            Expr::ArrayLiteral(elements) => {
                let elems: Vec<String> = elements.iter().map(|e| self.generate_expr(e)).collect();
                format!("vec![{}]", elems.join(", "))
            },
            Expr::Index(arr, idx) => {
                format!("{}[({} as usize)]", self.generate_expr(arr), self.generate_expr(idx))
            }
            Expr::MethodCall(obj, method, args) => {
                let args_str = args.iter().map(|a| self.generate_expr(a)).collect::<Vec<_>>().join(", ");
                format!("{}.{}({})", self.generate_expr(obj), method, args_str)
            },
            Expr::StructLiteral(name, fields) => {
                let fields_str = fields.iter().map(|(k, v)| format!("{}: {}", k, self.generate_expr(v))).collect::<Vec<_>>().join(", ");
                format!("{} {{ {} }}", name.replace(".", "::"), fields_str)
            },
            Expr::FieldAccess(obj, field) => {
                format!("{}.{}", self.generate_expr(obj), field)
            },
            Expr::TupleLiteral(elements) => {
                let elems: Vec<String> = elements.iter().map(|e| self.generate_expr(e)).collect();
                format!("({})", elems.join(", "))
            },
            Expr::TupleIndex(expr, idx) => {
                format!("{}.{}", self.generate_expr(expr), idx)
            },
            Expr::Binary(left, op, right) => {
                match op {
                    BinaryOp::Add => format!("{}.z_add({})", self.generate_expr_owned(left), self.generate_expr_owned(right)),
                    BinaryOp::Mul => format!("{}.z_mul({})", self.generate_expr_owned(left), self.generate_expr_owned(right)),
                    _ => {
                        let op_str = match op { 
                            BinaryOp::Sub => "-", BinaryOp::Div => "/", BinaryOp::Mod => "%",
                            BinaryOp::Eq => "==", BinaryOp::Neq => "!=", 
                            BinaryOp::Gt => ">", BinaryOp::Lt => "<",
                            BinaryOp::Gte => ">=", BinaryOp::Lte => "<=",
                            BinaryOp::And => "&&", BinaryOp::Or => "||",
                            BinaryOp::BitAnd => "&", BinaryOp::BitOr => "|", BinaryOp::BitXor => "^",
                            BinaryOp::Shl => "<<", BinaryOp::Shr => ">>",
                            _ => unreachable!()
                        };
                        format!("({} {} {})", self.generate_expr(left), op_str, self.generate_expr(right))
                    }
                }
            },
            Expr::Call(n, a, _awaited) => {
                if n == "println" {
                    if a.is_empty() { return "println!()".to_string(); }
                    return format!("println!(\"{{}}\", {})", a.iter().map(|x| self.generate_expr(x)).collect::<Vec<_>>().join(", "));
                }
                if n == "print" {
                    if a.is_empty() { return "print!()".to_string(); }
                    return format!("print!(\"{{}}\", {})", a.iter().map(|x| self.generate_expr(x)).collect::<Vec<_>>().join(", "));
                }
                if n == "input" || n == "inputln" {
                    return format!("{}({}){}", n, a.iter().map(|x| self.generate_expr_as_string(x)).collect::<Vec<_>>().join(", "), ".await");
                }
                if n == "__rust__" {
                    if let Some(Expr::Literal(Literal::Str(s))) = a.get(0) {
                        return s.clone();
                    }
                }
                if n == "error" && !a.is_empty() {
                    return format!("Err({}.to_string())", self.generate_expr(&a[0]));
                }
                let func_name_only = n.split("::").last().unwrap_or(n);
                let await_suffix = if self.pure_functions.contains(func_name_only) { "" } else { ".await" };
                format!("{}({}){}", n, a.iter().map(|x| self.generate_expr_owned(x)).collect::<Vec<_>>().join(", "), await_suffix)
            },
            Expr::InlineRust(code) => {
                code.clone()
            },
            Expr::Spawn(e) => {
                if self.in_scope {
                    format!("_zet_handles.push(tokio::spawn(async move {{ {} }}))", self.generate_expr(e))
                } else {
                    format!("tokio::spawn(async move {{ {} }})", self.generate_expr(e))
                }
            },
            Expr::Await(e) => format!("{}.await", self.generate_expr(e)),
            Expr::Try(e) => format!("{}?", self.generate_expr(e)),
            Expr::Catch(expr, fallback) => format!("{}.unwrap_or_else(|_| {{ {} }})", self.generate_expr(expr), self.generate_expr(fallback)),
        }
    }

    fn map_type(&self, t: &TypeRef) -> String { 
        match t { 
            TypeRef::Void => "()".to_string(), 
            TypeRef::Integer => "i64".to_string(), 
            TypeRef::Float => "f64".to_string(),
            TypeRef::Bool => "bool".to_string(),
            TypeRef::Char => "char".to_string(),
            TypeRef::Byte => "u8".to_string(),
            TypeRef::String => "String".to_string(),
            TypeRef::Untrusted => "Untrusted".to_string(),
            TypeRef::Result(inner) => format!("Result<{}, String>", self.map_type(inner)),
            TypeRef::Array(inner) => format!("Vec<{}>", self.map_type(inner)),
            TypeRef::Tuple(types) => format!("({})", types.iter().map(|t| self.map_type(t)).collect::<Vec<_>>().join(", ")),
            TypeRef::Custom(name) => name.replace(".", "::"),
        } 
    }

    fn map_assign_op(&self, op: &AssignOp) -> &str {
        match op {
            AssignOp::Assign => "=",
            AssignOp::AddAssign => "+=",
            AssignOp::SubAssign => "-=",
            AssignOp::MulAssign => "*=",
            AssignOp::DivAssign => "/=",
            AssignOp::ModAssign => "%=",
        }
    }
}
