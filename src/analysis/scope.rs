use crate::ast::*;
use std::collections::HashSet;

pub struct ScopeAnalyzer {
    defined_vars: HashSet<String>,
}

impl ScopeAnalyzer {
    pub fn new() -> Self {
        Self { defined_vars: HashSet::new() }
    }

    pub fn analyze(&mut self, func: &FunctionDef) -> Result<(), String> {
        self.defined_vars.clear();
        for param in &func.params {
            self.defined_vars.insert(param.name.clone());
        }
        self.visit_block(&func.body)
    }

    fn visit_block(&mut self, block: &Block) -> Result<(), String> {
        let backup = self.defined_vars.clone();
        for stmt in &block.statements {
            self.visit_stmt(stmt)?;
        }
        self.defined_vars = backup;
        Ok(())
    }

    fn visit_stmt(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let(l) => {
                self.visit_expr(&l.value)?;
                self.defined_vars.insert(l.name.clone());
            }
            Statement::Assign { name, value } => {
                if !self.defined_vars.contains(name) {
                    return Err(format!("Undefined variable: {}", name));
                }
                self.visit_expr(value)?;
            }
            Statement::If { condition, then_block, else_block } => {
                self.visit_expr(condition)?;
                self.visit_block(then_block)?;
                if let Some(else_b) = else_block { self.visit_block(else_b)?; }
            }
            Statement::While { condition, body } => {
                self.visit_expr(condition)?;
                self.visit_block(body)?;
            }
            Statement::For { var, start, end, step, body } => {
                self.visit_expr(start)?;
                self.visit_expr(end)?;
                if let Some(s) = step { self.visit_expr(s)?; } 
                self.defined_vars.insert(var.clone());
                self.visit_block(body)?;
            }
            Statement::ScopeBlock { body, .. } => self.visit_block(body)?,
            Statement::ValidateBlock { target, success_scope, .. } => {
                if !self.defined_vars.contains(target) {
                    return Err(format!("Undefined variable in validate: {}", target));
                }
                self.visit_block(success_scope)?;
            }
            Statement::ExprStmt(expr) => self.visit_expr(expr)?,
            Statement::Return(Some(e)) => self.visit_expr(e)?,
            Statement::Return(None) => {},
        }
        Ok(())
    }

    fn visit_expr(&self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Identifier(name) => {
                if !self.defined_vars.contains(name) {
                    return Err(format!("Undefined variable used: {}", name));
                }
            }
            Expr::Binary(l, _, r) => { self.visit_expr(l)?; self.visit_expr(r)?; }
            Expr::Call(_, args) => { for arg in args { self.visit_expr(arg)?; } }
            Expr::Spawn(e) => self.visit_expr(e)?,
            Expr::Await(e) => self.visit_expr(e)?,
            Expr::Infra(call) => { for arg in &call.args { self.visit_expr(arg)?; } }
            Expr::JsonField(source, _) => self.visit_expr(source)?,
            // YENİ: Array ve Index içini gezme
            Expr::ArrayLiteral(elems) => { for e in elems { self.visit_expr(e)?; } }
            Expr::Index(arr, idx) => { self.visit_expr(arr)?; self.visit_expr(idx)?; }
            _ => {}
        }
        Ok(())
    }
}