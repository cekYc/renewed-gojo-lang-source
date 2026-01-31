use crate::ast::*;

pub struct TaintAnalyzer;

impl TaintAnalyzer {
    pub fn check(func: &FunctionDef) -> Result<(), String> { Self::visit_block(&func.body) }

    fn visit_block(block: &Block) -> Result<(), String> { 
        for stmt in &block.statements { Self::visit_stmt(stmt)?; } 
        Ok(()) 
    }

    fn visit_stmt(stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let(l) => Self::visit_expr(&l.value),
            Statement::Assign { value, .. } => Self::visit_expr(value),
            Statement::If { condition, then_block, else_block } => { 
                Self::visit_expr(condition)?; 
                Self::visit_block(then_block)?; 
                if let Some(b) = else_block { Self::visit_block(b)?; } 
                Ok(()) 
            },
            Statement::While { condition, body } => { 
                Self::visit_expr(condition)?; 
                Self::visit_block(body)?; 
                Ok(()) 
            },
            Statement::For { start, end, step, body, .. } => {
                Self::visit_expr(start)?;
                Self::visit_expr(end)?;
                if let Some(s) = step { Self::visit_expr(s)?; }
                Self::visit_block(body)?;
                Ok(())
            },
            Statement::ScopeBlock { body, .. } => Self::visit_block(body),
            Statement::ValidateBlock { success_scope, .. } => Self::visit_block(success_scope),

            Statement::ExprStmt(e) | Statement::Return(Some(e)) => Self::visit_expr(e),
            Statement::Return(None) => Ok(()),
        }
    }

    fn visit_expr(expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Binary(l, _, r) => { Self::visit_expr(l)?; Self::visit_expr(r) },
            Expr::Call(_, args) => { for arg in args { Self::visit_expr(arg)?; } Ok(()) },
            Expr::Spawn(e) | Expr::Await(e) => Self::visit_expr(e),
            Expr::Infra(call) => { for arg in &call.args { Self::visit_expr(arg)?; } Ok(()) },
            Expr::JsonField(source, _) => Self::visit_expr(source),
            // YENİ: Array ve Index taint kontrolü
            Expr::ArrayLiteral(elems) => { for e in elems { Self::visit_expr(e)?; } Ok(()) }
            Expr::Index(arr, idx) => { Self::visit_expr(arr)?; Self::visit_expr(idx)?; Ok(()) }
            _ => Ok(()),
        }
    }
}