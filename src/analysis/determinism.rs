use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SymbolTable { pub functions: HashMap<String, FunctionDef> }

pub struct DeterminismAnalyzer;

impl DeterminismAnalyzer {
    pub fn check(func: &FunctionDef, _symbols: &SymbolTable) -> Result<(), String> {
        if let Purity::Deterministic = func.purity { if !Self::is_block_pure(&func.body) { return Err(format!("Impure function: {}", func.name)); } } Ok(())
    }
    fn is_block_pure(block: &Block) -> bool { block.statements.iter().all(Self::is_stmt_pure) }
    fn is_stmt_pure(stmt: &Statement) -> bool {
        match stmt {
            Statement::Let(l) => Self::is_expr_pure(&l.value),
            Statement::Assign { value, .. } => Self::is_expr_pure(value),
            Statement::If { condition, then_block, else_block } => Self::is_expr_pure(condition) && Self::is_block_pure(then_block) && else_block.as_ref().map(|b| Self::is_block_pure(b)).unwrap_or(true),
            Statement::While { condition, body } => Self::is_expr_pure(condition) && Self::is_block_pure(body),
            Statement::For { start, end, step, body, .. } => {
                Self::is_expr_pure(start) && 
                Self::is_expr_pure(end) && 
                step.as_ref().map(|s| Self::is_expr_pure(s)).unwrap_or(true) && 
                Self::is_block_pure(body)
            },
            Statement::ScopeBlock { body, .. } => Self::is_block_pure(body),
            Statement::ValidateBlock { success_scope, .. } => Self::is_block_pure(success_scope),
            Statement::ExprStmt(expr) | Statement::Return(Some(expr)) => Self::is_expr_pure(expr),
            Statement::Return(None) => true,
        }
    }
    fn is_expr_pure(expr: &Expr) -> bool {
        match expr {
            Expr::Literal(_) | Expr::Identifier(_) => true,
            Expr::Binary(l, _, r) => Self::is_expr_pure(l) && Self::is_expr_pure(r),
            Expr::Call(_, args) => args.iter().all(Self::is_expr_pure),
            Expr::JsonField(source, _) => Self::is_expr_pure(source),
            // YENİ: Array ve Index kontrolü eklendi
            Expr::ArrayLiteral(elems) => elems.iter().all(Self::is_expr_pure),
            Expr::Index(arr, idx) => Self::is_expr_pure(arr) && Self::is_expr_pure(idx),
            
            Expr::Spawn(_) | Expr::Await(_) | Expr::Infra(_) => false,
        }
    }
}