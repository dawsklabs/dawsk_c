mod abort;
mod args;
mod ast;
mod color;
mod lexer;
mod macros;
mod parser;
mod module;
mod reports;
mod source;

use std::sync::Arc;

use reports::ReportBag;
use source::SourceMap;

use crate::args::ArgumentParser;
use crate::ast::strings::StringPool;
use crate::macros::SyntaxContextTable;

use std::path::PathBuf;

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
    pub syntax_contexts: SyntaxContextTable,
    pub string_pool: StringPool,
}

#[cfg(feature = "bench")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "bench")]
    let _profiler = dhat::Profiler::new_heap();

    ArgumentParser::parse();

    let entry = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/src/main.awh");
    if !entry.exists() {
        eprintln!("error: '{}' not found", entry.display());
        std::process::exit(1);
    }

    let shared = Arc::new(SharedCtx::new());
    let mut compiler = Compiler {
        shared,
        sourcemap: SourceMap::new(),
        syntax_contexts: SyntaxContextTable::new(),
        string_pool: StringPool::new(),
    };

    let mut collector = module::ModuleCollector::new();
    let ast = collector.collect(&mut compiler, entry);

    if args::step_enabled(args::step::AST) {
        ast.visualize(&compiler.string_pool);
    }

    let _ = compiler
        .shared
        .reports
        .inner
        .lock()
        .unwrap()
        .print_all(&mut compiler.sourcemap);
}
