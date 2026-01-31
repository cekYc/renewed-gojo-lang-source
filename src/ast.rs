#[derive(Debug, Clone, PartialEq)]
pub enum TypeRef {
    Void, 
    Integer, 
    String, 
    Untrusted, 
    // YENİ: Dizi Tipi
    Array(Box<TypeRef>),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub param_type: TypeRef,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, 
    Eq, Neq, Gt, Lt,
    // YENİ: Büyük Eşit / Küçük Eşit
    Gte, Lte,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64), Str(String), Bool(bool),
}

#[derive(Debug, Clone)]
pub struct InfraConfig {
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct InfraCall {
    pub service: String,
    pub method: String,
    pub args: Vec<Expr>,
    pub config: InfraConfig,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Identifier(String),
    Literal(Literal),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Call(String, Vec<Expr>),
    Spawn(Box<Expr>),
    Await(Box<Expr>),
    Infra(InfraCall),
    JsonField(Box<Expr>, String),
    
    // YENİ: Liste Oluşturma [1, 2, 3]
    ArrayLiteral(Vec<Expr>),
    // YENİ: Listeden Okuma x[0]
    Index(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetStmt),
    Assign { name: String, value: Expr },
    If { condition: Expr, then_block: Block, else_block: Option<Block> },
    While { condition: Expr, body: Block },
    // FOR LOOP (Step ile birlikte)
    For { var: String, start: Expr, end: Expr, step: Option<Expr>, body: Block },
    ScopeBlock { name: String, body: Block },
    ValidateBlock { target: String, schema: String, on_fail: Box<Block>, success_scope: Box<Block> },
    ExprStmt(Expr), 
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Purity {
    Deterministic, Nondeterministic,
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub purity: Purity,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
    pub body: Block,
}