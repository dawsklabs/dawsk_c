use std::{collections::HashSet, path::PathBuf};

use crate::{
    ast::{ASTItem, AST},
    color::RED_COLOR,
    parser::Parser,
    reports::{Label, Report, ReportKind},
    Compiler,
};

pub struct ModuleCollector {
    seen: HashSet<PathBuf>,
}

impl ModuleCollector {
    pub fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    /// Einstiegspunkt — sammelt alles ab einer Wurzeldatei
    pub fn collect(&mut self, compiler: &mut Compiler, entry: PathBuf) -> AST {
        let mut ast = AST::new();
        self.collect_file(compiler, entry, &mut ast);
        ast
    }

    fn collect_file(&mut self, compiler: &mut Compiler, path: PathBuf, ast: &mut AST) {
        let path = match path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: could not resolve '{}': {}", path.display(), e);
                return;
            }
        };

        if !self.seen.insert(path.clone()) {
            return;
        }

        // base_dir direkt aus path ableiten — vor dem Parser-Block
        let base_dir = path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_owned(); // PathBuf damit es den Parser-Block überlebt

        let file_id = compiler.sourcemap.add_file(path);
        // path ist jetzt moved, kein Clone mehr nötig

        let mut includes = Vec::new();
        {
            let mut parser = Parser::new(compiler, file_id);
            while let Some(item) = parser.next_item() {
                if let ASTItem::Include(inc) = &item {
                    includes.push(inc.clone());
                    // kein path.clone() mehr nötig
                }
                ast.add_item(item);
            }
        } // parser wird hier gedroppt → compiler wieder frei

        for inc in includes {
            for module in inc.modules.iter() {
                let name = compiler.string_pool.get(module.id).unwrap_or("").to_owned();
                if let Some(resolved) = self.resolve(&base_dir, &name) {
                    self.collect_file(compiler, resolved, ast);
                } else {
                    compiler.shared.reports.push(
                        Report::build(ReportKind::Error, module.span)
                            .with_message(format!("module '{}' not found", name))
                            .with_label(
                                Label::new(module.span)
                                    .with_message(format!("searched in '{}'", base_dir.display()))
                                    .with_color(RED_COLOR),
                            )
                            .finish(),
                    );
                }
            }
        }
    }

    /// Sucht erst nach datei.awh, dann nach folder/mod.awh
    fn resolve(&self, base_dir: &std::path::Path, name: &str) -> Option<PathBuf> {
        // Priorität 1: datei.awh
        let file = base_dir.join(name).with_extension("dawsk");
        if file.exists() {
            return Some(file);
        }

        // Priorität 2: name/mod.awh
        let mod_file = base_dir.join(name).join("mod.dawsk");
        if mod_file.exists() {
            return Some(mod_file);
        }

        None
    }
}
