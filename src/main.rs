mod abort;
mod args;
mod ast;
mod color;
mod modules;
mod reports;
mod source;
mod types;

use ast::lexer::Lexer;
use ast::parser::Parser;
use ast::scope::{NameInterner, ScopeCtx};
use ast::token::TokenKind;
use ast::traits::TraitCtx;
use ast::typechecker::TypeChecker;
use ast::{ASTItem, AST};
use reports::ReportBag;
use source::SourceMap;
use types::inference::InferCtx;
use types::{SymbolInterner, TyInterner};

pub struct Compiler {
    sourcemap: SourceMap,
    ty_interner: TyInterner,
    infer_ctx: InferCtx,
    trait_ctx: TraitCtx,
    scopes: ScopeCtx,
    name_interner: NameInterner,
    symbol_interner: SymbolInterner,
    reports: ReportBag,
}

fn main() {
    let mut sourcemap = SourceMap::new();

    let file_id = sourcemap.add_file(
        "main.awh".into(),
        std::fs::read_to_string("main.awh").unwrap(),
    );

    let mut compiler = Compiler {
        sourcemap,
        ty_interner: TyInterner::new(),
        infer_ctx: InferCtx::new(),
        trait_ctx: TraitCtx::new(),
        scopes: ScopeCtx::new(),
        name_interner: NameInterner::new(),
        symbol_interner: SymbolInterner::new(),
        reports: ReportBag::new(),
    };

    compiler
        .trait_ctx
        .gen_default_traits(&mut compiler.ty_interner);

    // Lexer
    let mut lexer = Lexer::new(&mut compiler, file_id);
    let mut tokens = Vec::new();

    loop {
        let t = lexer.next_token();
        tokens.push(t.clone());
        if t.kind == TokenKind::EOF || t.kind == TokenKind::Error {
            break;
        }
    }

    // Parser
    let mut parser = Parser::new(tokens.as_slice(), &mut compiler);

    let mut ast = AST::new();
    while let Some(stmt) = parser.next_stmt() {
        ast.add_item(ASTItem::Stmt(stmt));
    }

    ast.visualize();

    // TypeChecker
    let tc = TypeChecker::new();
    tc.check(&mut compiler, &ast);

    let _ = compiler.reports.print_all(&mut compiler.sourcemap);
}
