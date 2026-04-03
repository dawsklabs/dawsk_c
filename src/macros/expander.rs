use crate::ast::strings::StringId;
use crate::ast::{ASTMacroDecExpr, CaptureKind, MacroBodyToken, MacroPatternToken, RepKind};
use crate::lexer::token::{Token, TokenKind};
use crate::macros::{ExpansionId, SyntaxContext, SyntaxContextTable};
use crate::source::{SourceMap, Span};
use std::collections::HashMap;

// Captured tokens – entweder einzeln oder als Repetition
#[derive(Clone)]
enum CaptureValue {
    Single(Vec<Token>),
    Repeated(Vec<Vec<Token>>),
}

type Bindings = HashMap<StringId, CaptureValue>;

#[derive(Debug)]
pub enum ExpandError {
    NoMatch(String),
    RecursionLimit,
}

pub struct MacroExpander {
    pub macros: HashMap<StringId, ASTMacroDecExpr>,
    next_expansion: u32,
    depth: usize,
}

impl MacroExpander {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
            next_expansion: 0,
            depth: 0,
        }
    }

    pub fn register(&mut self, i: StringId, def: ASTMacroDecExpr) {
        self.macros.insert(i, def);
    }

    pub fn expand(
        &mut self,
        id: StringId,
        args: Vec<Token>,
        call_span: Span,
        call_ctx: SyntaxContext,
        source_map: &mut SourceMap,
        ctx_table: &mut SyntaxContextTable,
    ) -> Result<Vec<Token>, ExpandError> {
        if self.depth >= 64 {
            return Err(ExpandError::RecursionLimit);
        }

        let def = self
            .macros
            .get(&id)
            .ok_or_else(|| ExpandError::NoMatch(id.to_string()))?
            .clone();

        self.depth += 1;
        let expansion_id = ExpansionId(self.next_expansion);
        self.next_expansion += 1;

        for rule in &def.rules {
            if let Some(bindings) = Self::match_pattern(&rule.pattern, &args) {
                let result = self.expand_body(
                    &rule.body,
                    &bindings,
                    &call_span,
                    call_ctx,
                    expansion_id,
                    source_map,
                    ctx_table,
                );
                self.depth -= 1;
                return Ok(result);
            }
        }

        self.depth -= 1;
        Err(ExpandError::NoMatch(id.to_string()))
    }

    fn expand_body(
        &mut self,
        body: &[MacroBodyToken],
        bindings: &Bindings,
        call_span: &Span,
        call_ctx: SyntaxContext,
        expansion_id: ExpansionId,
        source_map: &mut SourceMap,
        ctx_table: &mut SyntaxContextTable,
    ) -> Vec<Token> {
        let new_ctx = ctx_table.push(call_ctx, expansion_id);
        let mut result = Vec::new();

        for tok in body {
            match tok {
                MacroBodyToken::Literal(kind, def_span) => {
                    let source = source_map.register_macro(*call_span, *def_span);
                    result.push(Token {
                        kind: kind.clone(),
                        span: Span::new(def_span.start, def_span.end, def_span.file_id, source),
                        ctx: new_ctx,
                    });
                }
                MacroBodyToken::Var(name_id, _) => {
                    if let Some(CaptureValue::Single(tokens)) = bindings.get(name_id) {
                        result.extend(tokens.iter().cloned());
                    }
                }
                MacroBodyToken::Repetition {
                    tokens: rep_body,
                    separator,
                    ..
                } => {
                    let count = bindings
                        .values()
                        .filter_map(|v| match v {
                            CaptureValue::Repeated(vec) => Some(vec.len()),
                            _ => None,
                        })
                        .next()
                        .unwrap_or(0);

                    for i in 0..count {
                        let iter_bindings: Bindings = bindings
                            .iter()
                            .filter_map(|(k, v)| match v {
                                CaptureValue::Repeated(vec) => vec
                                    .get(i)
                                    .map(|tokens: &Vec<Token>| (*k, CaptureValue::Single(tokens.clone()))),
                                other => Some((*k, other.clone())),
                            })
                            .collect();

                        result.extend(self.expand_body(
                            rep_body,
                            &iter_bindings,
                            call_span,
                            call_ctx,
                            expansion_id,
                            source_map,
                            ctx_table,
                        ));

                        if i + 1 < count {
                            if let Some(sep) = separator {
                                result.push(Token {
                                    kind: sep.clone(),
                                    span: *call_span,
                                    ctx: new_ctx,
                                });
                            }
                        }
                    }
                }
            }
        }

        result
    }

    fn match_pattern(pattern: &[MacroPatternToken], input: &[Token]) -> Option<Bindings> {
        let mut bindings = HashMap::new();
        let mut pos = 0;

        for pat in pattern {
            match pat {
                MacroPatternToken::Literal(kind) => {
                    if input.get(pos)?.kind != *kind {
                        return None;
                    }
                    pos += 1;
                }

                MacroPatternToken::Capture { name, kind, .. } => {
                    let (captured, consumed) = Self::match_capture(kind, &input[pos..])?;
                    bindings.insert(*name, CaptureValue::Single(captured));
                    pos += consumed;
                }

                MacroPatternToken::Repetition {
                    tokens: rep_pat,
                    separator,
                    kind,
                } => {
                    let mut rep_count = 0usize;

                    loop {
                        let remaining = &input[pos..];
                        match Self::match_rep_once(rep_pat, remaining) {
                            Some((sub_bindings, consumed)) => {
                                rep_count += 1;
                                for (k, v) in sub_bindings {
                                    let entry = bindings
                                        .entry(k)
                                        .or_insert_with(|| CaptureValue::Repeated(Vec::new()));
                                    if let CaptureValue::Repeated(vec) = entry {
                                        if let CaptureValue::Single(tokens) = v {
                                            vec.push(tokens);
                                        }
                                    }
                                }
                                pos += consumed;

                                if let Some(sep) = separator {
                                    if input.get(pos).map(|t| &t.kind) == Some(sep) {
                                        pos += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }
                            None => break,
                        }
                    }

                    if *kind == RepKind::OneOrMore && rep_count == 0 {
                        return None;
                    }
                }
            }
        }

        // Input muss vollständig konsumiert sein
        if pos == input.len() {
            Some(bindings)
        } else {
            None
        }
    }

    fn match_capture(kind: &CaptureKind, input: &[Token]) -> Option<(Vec<Token>, usize)> {
        match kind {
            CaptureKind::Ident => {
                let tok = input.first()?;
                if matches!(tok.kind, TokenKind::Identifier(_)) {
                    Some((vec![tok.clone()], 1))
                } else {
                    None
                }
            }
            CaptureKind::Literal => {
                let tok = input.first()?;
                if matches!(
                    tok.kind,
                    TokenKind::Integer(..)
                        | TokenKind::Float(..)
                        | TokenKind::String(_)
                        | TokenKind::Char(_)
                ) {
                    Some((vec![tok.clone()], 1))
                } else {
                    None
                }
            }
            // expr und stmt: alles bis zum nächsten Komma oder Ende
            // Das ist eine Vereinfachung – ein echter expr-Parser wäre besser
            CaptureKind::Expr | CaptureKind::Stmt => {
                let consumed = Self::scan_expr(input);
                if consumed == 0 {
                    return None;
                }
                Some((input[..consumed].to_vec(), consumed))
            }
            CaptureKind::Ty => {
                let consumed = Self::scan_ty(input);
                if consumed == 0 {
                    return None;
                }
                Some((input[..consumed].to_vec(), consumed))
            }
        }
    }

    /// Scannt Tokens für einen Ausdruck – stoppt bei Komma oder Ende
    /// respektiert dabei Klammer-Tiefe
    fn scan_expr(input: &[Token]) -> usize {
        let mut depth = 0usize;
        let mut i = 0;

        for tok in input {
            match tok.kind {
                TokenKind::LParen | TokenKind::LCurly | TokenKind::LBracket => depth += 1,
                TokenKind::RParen | TokenKind::RCurly | TokenKind::RBracket => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                TokenKind::Comma | TokenKind::Semicolon if depth == 0 => break,
                _ => {}
            }
            i += 1;
        }
        i
    }

    /// Scannt Tokens für einen Typ
    fn scan_ty(input: &[Token]) -> usize {
        // Vereinfachung: wie expr aber stoppt auch bei >
        let mut depth = 0usize;
        let mut i = 0;

        for tok in input {
            match tok.kind {
                TokenKind::LAngle | TokenKind::LParen => depth += 1,
                TokenKind::RAngle | TokenKind::RParen => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                TokenKind::Comma if depth == 0 => break,
                _ => {}
            }
            i += 1;
        }
        i
    }

    fn match_rep_once(pattern: &[MacroPatternToken], input: &[Token]) -> Option<(Bindings, usize)> {
        // Vereinfachte Version: matcht genau ein Vorkommen des Patterns
        let mut bindings = HashMap::new();
        let mut pos = 0;

        for pat in pattern {
            match pat {
                MacroPatternToken::Literal(kind) => {
                    if input.get(pos)?.kind != *kind {
                        return None;
                    }
                    pos += 1;
                }
                MacroPatternToken::Capture { name, kind, .. } => {
                    let (captured, consumed) = Self::match_capture(kind, &input[pos..])?;
                    bindings.insert(*name, CaptureValue::Single(captured));
                    pos += consumed;
                }
                _ => return None, // verschachtelte Repetitionen später
            }
        }

        Some((bindings, pos))
    }
}
