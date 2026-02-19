use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    ast::{scope::Symbol, ASTItem, AST},
    Compiler,
};

pub struct FileSystem {
    root: PathBuf,
    cache: HashMap<PathBuf, Vec<u8>>,
}

impl FileSystem {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cache: HashMap::new(),
        }
    }

    pub fn read(&mut self, path: &Path) -> Option<Vec<u8>> {
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        if let Some(content) = self.cache.get(&abs) {
            return Some(content.clone());
        }

        let bytes = std::fs::read(&abs).ok()?;
        self.cache.insert(abs.clone(), bytes.clone());
        Some(bytes)
    }
}

#[derive(Clone)]
pub struct Import {
    pub path: Vec<String>, // foo::bar
    pub alias: Option<String>,
}

pub struct Module {
    pub name: String,
    pub path: Vec<String>,
    pub items: Vec<ASTItem>,
    pub exports: HashMap<String, Symbol>,
    pub imports: Vec<Import>,
    pub symbols: HashMap<String, Symbol>,
}

impl Module {
    pub fn from_ast(ast: AST, path: Vec<String>) -> Self {
        Self {
            name: path.last().unwrap().clone(),
            path: path.clone(),
            items: ast.items,
            exports: HashMap::new(),
            imports: Vec::new(),
            symbols: HashMap::new(),
        }
    }
}

pub struct ModuleResolver {
    fs: FileSystem,
    modules: HashMap<String, Module>,
}

impl ModuleResolver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            fs: FileSystem::new(root),
            modules: HashMap::new(),
        }
    }

    pub fn get_module(&self, path: &[String]) -> Option<&Module> {
        let key = path.join("::");
        self.modules.get(&key)
    }

    pub fn collect_imports(&mut self, compiler: &mut Compiler) {
        let module_keys: Vec<String> = self.modules.keys().cloned().collect();

        for key in module_keys {
            self.resolve_imports_for_module(compiler, &key);
        }
    }

    // fn collect_exports(module: &mut Module) {
    //     for item in &module.items {
    //         match item {
    //             ASTItem::Struct(s) if s.pub_ => {
    //                 let sym = Symbol::Struct(s.name.clone());
    //                 module.exports.insert(s.name.clone(), sym.clone());
    //                 module.symbols.insert(s.name.clone(), sym);
    //             }
    //             ASTItem::Fn(f) if f.pub_ => {
    //                 let sym = Symbol::Fn(f.name.clone());
    //                 module.exports.insert(f.name.clone(), sym.clone());
    //                 module.symbols.insert(f.name.clone(), sym);
    //             }
    //             _ => {}
    //         }
    //     }
    // }

    // pub fn load_module(&mut self, compiler: &mut Compiler, module_path: Vec<String>) {
    //     let key = module_path.join("::");

    //     if self.modules.contains_key(&key) {
    //         return;
    //     }

    //     let file_path = PathBuf::from(module_path.join("/")).with_extension("awh");
    //     let bytes = self.fs.read(&file_path).expect("file not found");
    //     let _source = Source::new(bytes);

    //     let ast = parse_module(compiler);

    //     let module = Module::from_ast(ast, module_path);

    //     self.modules.insert(key, module);
    // }

    fn resolve_imports_for_module(&mut self, _compiler: &mut Compiler, module_key: &str) {
        let imports = {
            let module = self.modules.get(module_key).unwrap();
            module.imports.clone()
        };

        let current_path = {
            let module = self.modules.get(module_key).unwrap();
            module.path.clone()
        };

        let mut resolved = Vec::new();

        for import in imports {
            let normalized = match normalize_import_path(&current_path, &import.path) {
                Some(p) => p,
                None => continue,
            };

            let (module_path, symbol_name) = normalized.split_at(normalized.len() - 1);

            // self.load_module(compiler, module_path.to_vec());

            let key = module_path.join("::");
            if let Some(module) = self.modules.get(&key) {
                if let Some(sym) = module.exports.get(symbol_name[0].as_str()) {
                    let alias = import.alias.clone().unwrap_or(symbol_name[0].clone());
                    resolved.push((alias, sym.clone()));
                }
            }
        }

        let module = self.modules.get_mut(module_key).unwrap();
        for (name, sym) in resolved {
            module.symbols.insert(name, sym);
        }
    }

    pub fn resolve_all_imports(&mut self, compiler: &mut Compiler) {
        loop {
            let before = self.symbol_snapshot();
            self.collect_imports(compiler);
            let after = self.symbol_snapshot();

            if before == after {
                break;
            }
        }
    }

    fn symbol_snapshot(&self) -> usize {
        self.modules.values().map(|m| m.symbols.len()).sum()
    }
}

// pub fn parse_module(compiler: &mut Compiler) -> AST {
//     let mut lexer = Lexer::new(compiler);
//     let mut tokens = Vec::new();

//     loop {
//         let t = lexer.next_token();
//         tokens.push(t.clone());
//         if t.kind == TokenKind::EOF || t.kind == TokenKind::Error {
//             break;
//         }
//     }

//     let mut parser = Parser::new(tokens, compiler);
//     let mut ast = AST::new();

//     while let Some(stmt) = parser.next_stmt() {
//         ast.add_item(ASTItem::Stmt(stmt));
//     }

//     ast
// }

fn normalize_import_path(current: &[String], import: &[String]) -> Option<Vec<String>> {
    let mut result = Vec::new();
    let mut it = import.iter();

    match it.next()?.as_str() {
        "crate" => {
            // start from root
        }
        "super" => {
            if current.is_empty() {
                return None;
            }
            result.extend_from_slice(&current[..current.len() - 1]);
        }
        first => {
            // relative import
            result.extend_from_slice(current);
            result.push(first.to_string());
        }
    }

    for part in it {
        if part == "super" {
            result.pop()?;
        } else {
            result.push(part.clone());
        }
    }

    Some(result)
}
