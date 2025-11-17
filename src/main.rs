mod ast;
mod diagnostics;
mod text;

use ast::lexer::Lexer;
use ast::token::TokenKind;
use ast::AST;
use ast::parser::Parser;
use ast::eval::Evaluator;
use diagnostics::{
    Diagnostic, DiagnosticBag, DiagnosticBagCell, 
    DiagnosticKind, DiagnosticType, printer::Printer,
};
use text::{Source, file};

use std::{rc::Rc, cell::RefCell};

fn main() {
    file::set("main.awh");
    let source = Source::new(file::content());
    let diagnostics: DiagnosticBagCell = Rc::new(RefCell::new(DiagnosticBag::new()));
    let binding = diagnostics.borrow();
    let printer = Printer::new(&source, &binding.get());

    let mut lexer = Lexer::new();
    let mut tokens = Vec::new();

    loop {
        let t = lexer.next_token();
        tokens.push(t.clone());
        if t.kind == TokenKind::EOF {
            break; // Stop parsing when EOF is encountered
        }
        println!("{:?}", t);
    }

    let mut ast = AST::new();
    let mut parser = Parser::from_tokens(tokens.clone(), diagnostics.clone());

    loop {
        let s = parser.next_stmt();
        if !s.is_none() {
            ast.add_stmt(s.unwrap());
        } else {
            break; // Stop parsing when no more statements are available
        }
    }
    // while let Some(s) = parser.next_stmt() {
    //     ast.add_stmt(s);
    // }

    ast.visualize();
    
    let mut ev = Evaluator::new();
    let res = ev.eval_ast(&ast).unwrap();
    println!("Result: {:?}", res);

    println!("{}", diagnostics.borrow().render()); // Render and print all collected diagnostics
    println!("{}", printer.stringify());
}