mod abort;
mod args;
mod ast;
mod color;
// mod modules;
mod reports;
// mod resolver;
mod source;
// mod types;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use ast::parser::Parser;
// use ast::scope::{NameInterner /*, ScopeCtx */};
// use ast::traits::TraitCtx;
// use ast::typechecker::TypeChecker;
use ast::{ASTItem, AST};
use reports::ReportBag;
use source::SourceMap;
// use types::inference::InferCtx;
// use types::{/* SymbolInterner, */ TyInterner};

use crate::args::ArgumentParser;

#[derive(Default)]
pub struct SharedCtx {
    // pub name_interner: NameInterner, // Arc<RwLock<>> intern
    pub reports: ReportBag, // Arc<Mutex<>> intern
}

impl SharedCtx {
    pub fn new() -> Self {
        Self {
            reports: ReportBag::new(),
        }
    }
}

// Compiler hält dann:
pub struct Compiler {
    pub shared: Arc<SharedCtx>,
    pub sourcemap: SourceMap,
}

fn main() {
    ArgumentParser::parse();

    let mut sourcemap = SourceMap::new();

    let file_id = sourcemap.add_file(
        "main.awh".into(),
        std::fs::read_to_string("main.awh").unwrap(),
    );

    let shared = Arc::new(SharedCtx::new());

    let mut compiler = Compiler { shared, sourcemap };

    // compiler
    //     .trait_ctx
    //     .gen_default_traits(&mut compiler.ty_interner);

    // Parser
    let mut parser = Parser::new(&mut compiler, file_id);

    let mut ast = AST::new();
    while let Some(stmt) = parser.next_stmt() {
        ast.add_item(ASTItem::Stmt(stmt));
    }

    if args::step_enabled(args::step::AST) {
        ast.visualize(&parser.string_pool);
    }

    // TypeChecker
    // let tc = TypeChecker::new();
    // tc.check(&mut compiler, &ast);

    let _ = compiler
        .shared
        .reports
        .inner
        .lock()
        .unwrap()
        .print_all(&mut compiler.sourcemap);
}
