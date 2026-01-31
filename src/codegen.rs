use crate::ast::*;
use std::collections::HashSet;

pub struct Codegen { 
    indent_level: usize,
    pure_functions: HashSet<String>,
    // YENİ: Şu an hangi fonksiyondayız, o fonksiyon saf mı?
    is_current_func_pure: bool, 
}

impl Codegen {
    pub fn new() -> Self { 
        Self { 
            indent_level: 0,
            pure_functions: HashSet::new(),
            is_current_func_pure: false, 
        } 
    }

    fn indent(&self) -> String { "    ".repeat(self.indent_level) }

    fn get_runtime_preamble(&self) -> String {
        r#"
#![allow(dead_code, unused_imports, unused_variables, unused_parens, unused_mut)]
use std::time::Duration;
use std::io::{self, Write};
use serde_json::Value;

const RESET: &str = "\x1b[0m";
const CYAN: &str = "\x1b[36m";   
const GREEN: &str = "\x1b[32m";  
const MAGENTA: &str = "\x1b[35m";
const YELLOW: &str = "\x1b[33m"; 
const BLUE: &str = "\x1b[34m";
const RED: &str = "\x1b[31m";

struct DB;
impl DB {
    async fn log<T: std::fmt::Display>(msg: T) { println!("  {}[DB] Log: {}{}", CYAN, msg, RESET); }
}

struct Console;
impl Console {
    async fn read(prompt: String) -> String {
        print!("  {}[Console]  {}: {} ", BLUE, prompt, RESET);
        io::stdout().flush().unwrap();
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        buffer.trim().to_string()
    }
}

struct Util;
impl Util {
    #[inline(always)]
    async fn to_int(s: String) -> i64 { s.trim().parse::<i64>().unwrap_or(0) }
    #[inline(always)]
    async fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }
}

struct HTTP;
impl HTTP {
    async fn get(url: String) -> String {
        let client = reqwest::Client::builder().user_agent("GojoLang/1.0").build().unwrap();
        match client.get(&url).send().await {
            Ok(res) => res.text().await.unwrap_or_else(|e| format!("Error: {}", e)),
            Err(e) => format!("Error: {}", e)
        }
    }
}

trait Validate { fn validate(&self) -> Result<String, String>; }
impl Validate for String {
    fn validate(&self) -> Result<String, String> { Ok(self.clone()) }
}

// TRAITLER (Inline optimize edildi)
trait GojoAdd<Rhs> { type Output; fn g_add(self, rhs: Rhs) -> Self::Output; }
impl GojoAdd<i64> for i64 { type Output = i64; #[inline(always)] fn g_add(self, rhs: i64) -> i64 { self + rhs } }
impl GojoAdd<String> for String { type Output = String; #[inline(always)] fn g_add(self, rhs: String) -> String { self + &rhs } }
impl<'a> GojoAdd<&'a str> for String { type Output = String; #[inline(always)] fn g_add(self, rhs: &'a str) -> String { self + rhs } }
impl GojoAdd<i64> for String { type Output = String; #[inline(always)] fn g_add(self, rhs: i64) -> String { format!("{}{}", self, rhs) } }

trait GojoMul<Rhs> { type Output; fn g_mul(self, rhs: Rhs) -> Self::Output; }
impl GojoMul<i64> for i64 { type Output = i64; #[inline(always)] fn g_mul(self, rhs: i64) -> i64 { self * rhs } }
impl GojoMul<i64> for String { type Output = String; fn g_mul(self, rhs: i64) -> String { self.repeat(rhs as usize) } }
impl<'a> GojoMul<i64> for &'a str { type Output = String; fn g_mul(self, rhs: i64) -> String { self.repeat(rhs as usize) } }
"#.to_string()
    }

    pub fn generate(&mut self, functions: &Vec<FunctionDef>) -> String {
        self.pure_functions.clear();
        for func in functions {
            if let Purity::Deterministic = func.purity {
                self.pure_functions.insert(func.name.clone());
            }
        }

        let mut code = self.get_runtime_preamble();
        for func in functions {
            code.push_str(&self.generate_function(func));
        }
        if let Some(_) = functions.iter().find(|f| f.name == "main") {
             code.push_str(&self.generate_main_shim());
        }
        code
    }

    fn generate_function(&mut self, func: &FunctionDef) -> String {
        let real_func_name = if func.name == "main" { "user_main" } else { &func.name };
        let params = func.params.iter().map(|p| format!("{}: {}", p.name, self.map_type(&p.param_type))).collect::<Vec<_>>().join(", ");
        
        // Hangi fonksiyonda olduğumuzu kaydedelim
        let is_pure = self.pure_functions.contains(&func.name);
        self.is_current_func_pure = is_pure;
        
        let async_keyword = if is_pure { "" } else { "async " };

        let mut code = format!("pub {}fn {}({}) -> {} {{\n", async_keyword, real_func_name, params, self.map_type(&func.return_type));
        self.indent_level += 1;
        code.push_str(&self.generate_block(&func.body));
        self.indent_level -= 1;
        code.push_str("}\n\n");
        code
    }

    fn generate_main_shim(&self) -> String {
        r#"#[tokio::main] async fn main() { user_main("Internet".to_string()).await; tokio::time::sleep(std::time::Duration::from_millis(100)).await; }"#.to_string()
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
            Statement::Assign { name, value } => format!("{}{} = {};\n", indent, name, self.generate_expr(value)),
            Statement::ExprStmt(e) => format!("{}{};\n", indent, self.generate_expr(e)),
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
                let mut s = format!("{}{{\n", indent); 
                s.push_str(&format!("{}let mut {} = {};\n", self.indent(), var, self.generate_expr(start)));
                s.push_str(&format!("{}let _gojo_end = {};\n", self.indent(), self.generate_expr(end)));
                s.push_str(&format!("{}let _gojo_step = {};\n", self.indent(), step_expr));
                s.push_str(&format!("{}while (_gojo_step > 0 && {} < _gojo_end) || (_gojo_step < 0 && {} > _gojo_end) {{\n", self.indent(), var, var));
                self.indent_level += 1;
                s.push_str(&self.generate_block(body));
                
                // For döngüsü optimizasyonu
                if self.is_current_func_pure {
                     s.push_str(&format!("{}{} += _gojo_step;\n", self.indent(), var));
                } else {
                     s.push_str(&format!("{}{} = {}.g_add(_gojo_step);\n", self.indent(), var, var));
                }
                
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", indent));
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
                s.push_str(&self.generate_block(body));
                s.push_str(&format!("{}tokio::time::sleep(Duration::from_millis(50)).await;\n", self.indent()));
                self.indent_level -= 1;
                s.push_str(&format!("{}}}\n", indent));
                s
            }
            Statement::ValidateBlock { target, success_scope, .. } => {
                let mut s = format!("{}let {} = {}.validate().unwrap();\n", indent, target, target);
                s.push_str(&self.generate_block(success_scope));
                s
            }
            Statement::Return(Some(e)) => format!("{}return {};\n", indent, self.generate_expr(e)),
            Statement::Return(None) => format!("{}return;\n", indent),
        }
    }

    fn generate_expr_as_string(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(Literal::Str(_)) => self.generate_expr(expr),
            _ => format!("format!(\"{{}}\", {})", self.generate_expr(expr))
        }
    }

    fn generate_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Identifier(s) => s.clone(),
            Expr::Literal(l) => match l { Literal::Int(i) => i.to_string(), Literal::Str(s) => format!("\"{}\".to_string()", s), Literal::Bool(b) => b.to_string() },
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
            },
            // AKILLI HİBRİT SİSTEM 
            Expr::Binary(left, op, right) => {
                // Eğer fonksiyon SAF ise, direkt + - * / kullan (Native Hız)
                // Değilse .g_add() kullan (String birleştirme desteği için)
                if self.is_current_func_pure {
                     let op_str = match op { 
                        BinaryOp::Add => "+", BinaryOp::Sub => "-", BinaryOp::Mul => "*", BinaryOp::Div => "/",
                        BinaryOp::Eq => "==", BinaryOp::Neq => "!=", 
                        BinaryOp::Gt => ">", BinaryOp::Lt => "<",
                        BinaryOp::Gte => ">=", BinaryOp::Lte => "<=",
                    };
                    format!("({} {} {})", self.generate_expr(left), op_str, self.generate_expr(right))
                } else {
                    match op {
                        BinaryOp::Add => format!("{}.g_add({})", self.generate_expr(left), self.generate_expr(right)),
                        BinaryOp::Mul => format!("{}.g_mul({})", self.generate_expr(left), self.generate_expr(right)),
                        _ => {
                             let op_str = match op { 
                                 BinaryOp::Sub => "-", BinaryOp::Div => "/",
                                BinaryOp::Eq => "==", BinaryOp::Neq => "!=", 
                                BinaryOp::Gt => ">", BinaryOp::Lt => "<",
                                BinaryOp::Gte => ">=", BinaryOp::Lte => "<=",
                                _ => unreachable!()
                            };
                            format!("({} {} {})", self.generate_expr(left), op_str, self.generate_expr(right))
                        }
                    }
                }
            },
            Expr::Call(n, a) => {
                let await_suffix = if self.pure_functions.contains(n) { "" } else { ".await" };
                format!("{}({}){}", n.replace(".", "::"), a.iter().map(|x| self.generate_expr(x)).collect::<Vec<_>>().join(", "), await_suffix)
            },
            Expr::Spawn(e) => format!("tokio::spawn(async move {{ {} }})", self.generate_expr(e)),
            Expr::Await(e) => format!("{}.await", self.generate_expr(e)),
        }
    }

    fn map_type(&self, t: &TypeRef) -> String { 
        match t { 
            TypeRef::Void => "()".to_string(), 
            TypeRef::Integer => "i64".to_string(), 
            TypeRef::String => "String".to_string(),
            TypeRef::Array(inner) => format!("Vec<{}>", self.map_type(inner)),
            _ => "String".to_string() 
        } 
    }
}