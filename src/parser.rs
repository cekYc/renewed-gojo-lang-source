use nom::{
    bytes::complete::{tag, take_while},
    character::complete::{alpha1, char, digit1, multispace1, not_line_ending},
    combinator::{map, map_res, opt, recognize, value, verify},
    sequence::{delimited, pair, preceded, tuple, terminated},
    branch::alt,
    multi::{separated_list0, many0, many1},
    IResult,
};
use crate::ast::*;

fn sp<'a, E: nom::error::ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    value((), many0(alt((value((), multispace1), value((), pair(tag("//"), not_line_ending))))))(input)
}

fn ws<'a, F: 'a, O, E: nom::error::ParseError<&'a str>>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where F: FnMut(&'a str) -> IResult<&'a str, O, E> {
    delimited(sp, inner, sp)
}

// YENİ: [1, 2, 3] okuyan fonksiyon
fn parse_array_literal(input: &str) -> IResult<&str, Expr> {
    map(delimited(
        ws(char('[')),
        separated_list0(ws(char(',')), parse_expr),
        ws(char(']'))
    ), Expr::ArrayLiteral)(input)
}

fn identifier(input: &str) -> IResult<&str, String> {
    verify(
        map(recognize(pair(alt((alpha1, tag("_"))), take_while(|c: char| c.is_alphanumeric() || c == '_'))), |s: &str| s.to_string()),
        |s: &String| !["if", "else", "let", "while", "for", "in", "by", "scope", "spawn", "call", "json", "validate", "true", "false", "return", "nondeterministic", "deterministic", "fn", "Untrusted", "i64", "Void"].contains(&s.as_str())
    )(input)
}

fn dot_identifier(input: &str) -> IResult<&str, String> {
    map(recognize(pair(identifier, pair(char('.'), identifier))), |s: &str| s.to_string())(input)
}

fn number(input: &str) -> IResult<&str, i64> {
    map_res(recognize(pair(opt(char('-')), digit1)), |s: &str| s.parse::<i64>())(input)
}

fn string_literal(input: &str) -> IResult<&str, String> {
    map(delimited(char('"'), take_while(|c| c != '"'), char('"')), |s: &str| s.to_string())(input)
}

fn parse_json_field(input: &str) -> IResult<&str, Expr> {
    map(tuple((
        ws(tag("json")), ws(char('(')), parse_expr, ws(char(',')), ws(string_literal), ws(char(')'))
    )), |(_, _, source, _, key, _)| Expr::JsonField(Box::new(source), key))(input)
}

fn parse_infra_expr(input: &str) -> IResult<&str, Expr> {
    map(tuple((
        ws(tag("call")), identifier, ws(char('.')), identifier, ws(char('(')), separated_list0(ws(char(',')), parse_expr), ws(char(')')), 
        ws(char('{')), ws(tag("timeout")), ws(char(':')), number, ws(char('}'))
    )), |(_, s, _, m, _, a, _, _, _, _, t, _)| Expr::Infra(InfraCall { service: s, method: m, args: a, config: InfraConfig { timeout_ms: t as u64 } }))(input)
}

fn parse_spawn(input: &str) -> IResult<&str, Expr> {
    map(preceded(ws(tag("spawn")), parse_expr), |e| Expr::Spawn(Box::new(e)))(input)
}

fn parse_call_expr(input: &str) -> IResult<&str, Expr> {
    map(tuple((
        opt(ws(tag("call"))), 
        alt((dot_identifier, identifier)), 
        ws(char('(')), 
        separated_list0(ws(char(',')), parse_expr), 
        ws(char(')'))
    )), |(_, n, _, a, _)| Expr::Call(n, a))(input)
}

fn parse_primary(input: &str) -> IResult<&str, Expr> {
    alt((
        parse_spawn,
        parse_infra_expr, 
        parse_json_field,
        parse_call_expr,  
        parse_array_literal, // YENİ: Liste
        map(number, |n| Expr::Literal(Literal::Int(n))),
        map(string_literal, |s| Expr::Literal(Literal::Str(s))),
        map(tag("true"), |_| Expr::Literal(Literal::Bool(true))),
        map(tag("false"), |_| Expr::Literal(Literal::Bool(false))),
        map(identifier, Expr::Identifier),
        delimited(ws(char('(')), parse_expr, ws(char(')'))),
    ))(input)
}

// YENİ: x[0] gibi index erişimlerini çözen atom
fn parse_atom(input: &str) -> IResult<&str, Expr> {
    let (input, mut expr) = parse_primary(input)?;
    let (input, indices) = many0(delimited(ws(char('[')), parse_expr, ws(char(']'))))(input)?;
    for idx in indices {
        expr = Expr::Index(Box::new(expr), Box::new(idx));
    }
    Ok((input, expr))
}

fn parse_factor(input: &str) -> IResult<&str, Expr> {
    let (input, mut left) = parse_atom(input)?;
    let (input, ops) = many0(pair(ws(alt((char('*'), char('/')))), parse_atom))(input)?;
    for (op_char, right) in ops {
        let op = match op_char { '*' => BinaryOp::Mul, '/' => BinaryOp::Div, _ => unreachable!() };
        left = Expr::Binary(Box::new(left), op, Box::new(right));
    }
    Ok((input, left))
}

fn parse_term(input: &str) -> IResult<&str, Expr> {
    let (input, mut left) = parse_factor(input)?;
    let (input, ops) = many0(pair(ws(alt((char('+'), char('-')))), parse_factor))(input)?;
    for (op_char, right) in ops {
        let op = match op_char { '+' => BinaryOp::Add, '-' => BinaryOp::Sub, _ => unreachable!() };
        left = Expr::Binary(Box::new(left), op, Box::new(right));
    }
    Ok((input, left))
}

fn parse_expr(input: &str) -> IResult<&str, Expr> {
    let (input, mut left) = parse_term(input)?;
    let (input, ops) = many0(pair(ws(alt((
        tag("=="), tag("!="), 
        tag(">="), tag("<="), // YENİLER (Sıralama önemli)
        tag(">"), tag("<")
    ))), parse_term))(input)?;
    
    for (op_str, right) in ops {
        let op = match op_str { 
            "==" => BinaryOp::Eq, "!=" => BinaryOp::Neq, 
            ">=" => BinaryOp::Gte, "<=" => BinaryOp::Lte,
            ">" => BinaryOp::Gt, "<" => BinaryOp::Lt, 
            _ => unreachable!() 
        };
        left = Expr::Binary(Box::new(left), op, Box::new(right));
    }
    Ok((input, left))
}

fn parse_let(input: &str) -> IResult<&str, Statement> {
    map(tuple((ws(tag("let")), identifier, ws(char('=')), parse_expr)), |(_, n, _, v)| Statement::Let(LetStmt { name: n, value: v }))(input)
}

fn parse_assign(input: &str) -> IResult<&str, Statement> {
    map(tuple((identifier, ws(char('=')), parse_expr)), |(n, _, v)| Statement::Assign { name: n, value: v })(input)
}

fn parse_return(input: &str) -> IResult<&str, Statement> {
    map(tuple((
        ws(tag("return")),
        opt(parse_expr)
    )), |(_, expr)| Statement::Return(expr))(input)
}

fn parse_while(input: &str) -> IResult<&str, Statement> {
    map(tuple((ws(tag("while")), parse_expr, ws(char('{')), parse_block_content, ws(char('}')))), |(_, cond, _, stmts, _)| Statement::While { condition: cond, body: Block { statements: stmts } })(input)
}

fn parse_for(input: &str) -> IResult<&str, Statement> {
    map(tuple((
        ws(tag("for")), identifier, ws(tag("in")), parse_expr, ws(tag("..")), parse_expr,
        opt(preceded(ws(tag("by")), parse_expr)),
        ws(char('{')), parse_block_content, ws(char('}'))
    )), |(_, var, _, start, _, end, step, _, stmts, _)| 
        Statement::For { var, start, end, step, body: Block { statements: stmts } }
    )(input)
}

fn parse_if(input: &str) -> IResult<&str, Statement> {
    map(tuple((
        ws(tag("if")), parse_expr, ws(char('{')), parse_block_content, ws(char('}')),
        opt(preceded(ws(tag("else")), alt((
            map(parse_if, |stmt| match stmt { Statement::If { .. } => vec![stmt], _ => unreachable!() }),
            delimited(ws(char('{')), parse_block_content, ws(char('}')))
        ))))
    )), |(_, cond, _, then_stmts, _, else_stmts)| {
        Statement::If { condition: cond, then_block: Block { statements: then_stmts }, else_block: else_stmts.map(|stmts| Block { statements: stmts }) }
    })(input)
}

fn parse_scope(input: &str) -> IResult<&str, Statement> {
    map(tuple((ws(tag("scope")), identifier, ws(char('{')), parse_block_content, ws(char('}')))), |(_, n, _, s, _)| Statement::ScopeBlock { name: n, body: Block { statements: s } })(input)
}

fn parse_validate(input: &str) -> IResult<&str, Statement> {
    map(tuple((ws(tag("validate")), identifier, ws(char('{')), ws(tag("success")), ws(char(':')), ws(char('{')), parse_block_content, ws(char('}')), ws(char('}')))), 
    |(_, t, _, _, _, _, s, _, _)| Statement::ValidateBlock { target: t, schema: "Schema".to_string(), on_fail: Box::new(Block{statements:vec![]}), success_scope: Box::new(Block { statements: s }) })(input)
}

fn parse_statement(input: &str) -> IResult<&str, Statement> {
    alt((parse_let, parse_if, parse_while, parse_for, parse_scope, parse_validate, parse_assign, parse_return, map(terminated(parse_expr, opt(ws(char(';')))), |e| Statement::ExprStmt(e))))(input)
}

fn parse_block_content(input: &str) -> IResult<&str, Vec<Statement>> { many0(ws(parse_statement))(input) }

fn parse_type(input: &str) -> IResult<&str, TypeRef> {
    ws(alt((
        map(tag("Untrusted"), |_| TypeRef::Untrusted),
        map(tag("i64"), |_| TypeRef::Integer),
        map(tag("Void"), |_| TypeRef::Void),
        // YENİ: Dizi Tipi
        map(tuple((ws(tag("Array")), ws(char('<')), parse_type, ws(char('>')))), |(_, _, t, _)| TypeRef::Array(Box::new(t))),
        map(identifier, |s| TypeRef::Custom(s))
    )))(input)
}

fn parse_function(input: &str) -> IResult<&str, FunctionDef> {
    map(tuple((
        ws(alt((map(tag("deterministic"), |_| Purity::Deterministic), map(tag("nondeterministic"), |_| Purity::Nondeterministic)))),
        ws(tag("fn")), ws(identifier), ws(char('(')),
        separated_list0(ws(char(',')), map(tuple((ws(identifier), ws(char(':')), parse_type)), |(n, _, t)| Param { name: n, param_type: t })),
        ws(char(')')), 
        opt(preceded(ws(tag("->")), parse_type)), 
        ws(char('{')), parse_block_content, ws(char('}'))
    )), |(p, _, n, _, params, _, ret, _, stmts, _)| FunctionDef { name: n, purity: p, params, return_type: ret.map(|r| r).unwrap_or(TypeRef::Void), body: Block { statements: stmts } })(input)
}

pub fn parse_program(input: &str) -> IResult<&str, Vec<FunctionDef>> {
    many1(ws(parse_function))(input)
}