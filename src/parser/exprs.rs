use crate::{ast::{ASTBinaryOperator, ASTBinaryOperatorKind, ASTExpr, ASTUnaryOperator, ASTUnaryOperatorKind}, lexer::token::{Keyword, TokenKind}, parser::Parser, source::{Span, SpanSource}};

impl<'a> Parser<'a> {
    pub(super) fn parse_assignment(&mut self, target: ASTExpr) -> Result<ASTExpr,()> {
        let op_token = self.consume();
        let op_kind = match op_token.kind {
            TokenKind::Equals => ASTBinaryOperatorKind::Assign,
            TokenKind::PlusEquals => ASTBinaryOperatorKind::AddAssign,
            TokenKind::MinusEquals => ASTBinaryOperatorKind::SubtractAssign,
            TokenKind::AsteriskEquals => ASTBinaryOperatorKind::MultiplyAssign,
            TokenKind::SlashEquals => ASTBinaryOperatorKind::DivideAssign,
            _ => unreachable!(),
        };
        let rhs = self.parse_binary_expr(0)?;
        let span = Span::merge(target.span, rhs.span);
        Ok(ASTExpr::assignment(target, op_kind, rhs, span))
    }

    fn parse_unary_expr(&mut self) -> Result<ASTExpr, ()> {
        let mut ops = Vec::new();

        // 1. Sammle ALLE Prefix-Operatoren
        loop {
            match self.peek(0).kind {
                TokenKind::DoubleAnd => {
                    let tok = self.consume();

                    // Splitte in zwei '&'
                    ops.push(ASTUnaryOperator::new(
                        ASTUnaryOperatorKind::Ref,
                        Span::new(
                            tok.span.start,
                            tok.span.start + 1,
                            tok.span.end,
                            SpanSource::Source,
                        ),
                    ));

                    ops.push(ASTUnaryOperator::new(
                        ASTUnaryOperatorKind::Ref,
                        Span::new(
                            tok.span.start + 1,
                            tok.span.start,
                            tok.span.end,
                            SpanSource::Source,
                        ),
                    ));
                }
                TokenKind::And => {
                    let span = self.consume().span;
                    if self.peek(0).kind == TokenKind::Keyword(Keyword::Mut) {
                        self.advance(1);
                        ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::RefMut, span));
                    } else {
                        ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Ref, span));
                    }
                }
                TokenKind::Asterisk => {
                    let span = self.consume().span;
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Deref, span));
                }
                TokenKind::Minus => {
                    let span = self.consume().span;
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Negate, span));
                }
                TokenKind::Exclamation => {
                    let span = self.consume().span;
                    ops.push(ASTUnaryOperator::new(ASTUnaryOperatorKind::Not, span));
                }
                _ => break,
            }
        }

        // 2. Parse das eigentliche Operand
        let mut expr = self.parse_primary_expr()?;

        // 3. Wende Operatoren RÜCKWÄRTS an
        for op in ops.into_iter().rev() {
            let span = Span::merge(op.span, expr.span);
            expr = ASTExpr::unary(op, expr, span);
        }

        Ok(expr)
    }

    fn parse_cast_expr(&mut self) -> Result<ASTExpr, ()> {
        let mut expr = self.parse_unary_expr()?;

        while self.peek(0).kind == TokenKind::Keyword(Keyword::As) {
            self.consume(); // 'as'

            let ty = self.parse_type()?;
            let end = self.backpeek(1).span;
            let span = Span::merge(expr.span, end);

            expr = ASTExpr::cast(expr, ty, span);
        }

        Ok(expr)
    }

    pub(super) fn parse_binary_expr(&mut self, precedence: u8) -> Result<ASTExpr, ()> {
        let mut left = self.parse_cast_expr()?;

        loop {
            let op_token = self.peek(0);
            let op_kind = match &op_token.kind {
                TokenKind::Plus => ASTBinaryOperatorKind::Add,
                TokenKind::Minus => ASTBinaryOperatorKind::Subtract,
                TokenKind::Asterisk => ASTBinaryOperatorKind::Multiply,
                TokenKind::Slash => ASTBinaryOperatorKind::Divide,
                TokenKind::Percent => ASTBinaryOperatorKind::Remainder,

                // Vergleichsoperatoren
                TokenKind::DoubleEquals => ASTBinaryOperatorKind::Equal,
                TokenKind::ExclamationEquals => ASTBinaryOperatorKind::NotEqual,

                TokenKind::LAngle => ASTBinaryOperatorKind::Less,
                TokenKind::LAngleEquals => ASTBinaryOperatorKind::LessEqual,
                TokenKind::DoubleLAngle => ASTBinaryOperatorKind::LBitShift,

                TokenKind::RAngle => ASTBinaryOperatorKind::Greater,
                TokenKind::RAngleEquals => ASTBinaryOperatorKind::GreaterEqual,
                TokenKind::DoubleRAngle => ASTBinaryOperatorKind::RBitShift,

                TokenKind::And => ASTBinaryOperatorKind::BitAnd,
                TokenKind::Pipe => ASTBinaryOperatorKind::BitOr,
                TokenKind::Caret => ASTBinaryOperatorKind::LogicXor,

                TokenKind::DoubleAnd => ASTBinaryOperatorKind::LogicAnd,
                TokenKind::DoublePipe => ASTBinaryOperatorKind::LogicOr,

                _ => break,
            };

            let op = ASTBinaryOperator::new(op_kind, op_token.span);
            let op_prec = op.prec();
            if op_prec <= precedence {
                break;
            }

            // Consume Header Token
            self.advance(1);

            // right hand side Expression
            let right = self.parse_binary_expr(op_prec)?;
            let span = Span::merge(left.span, right.span);
            left = ASTExpr::binary(left, right, op, span);
        }

        Ok(left)
    }

    /// Parst einen primären Ausdruck + alle Postfix-Operatoren.
    /// Postfix: .field, .method(args), (args), [index], ::segment
    fn parse_primary_expr(&mut self) -> Result<ASTExpr, ()> {
        let mut expr = self.parse_atom()?;

        // Suffix-Loop
        loop {
            match self.peek(0).kind {
                // Feldzugriff oder Methodenaufruf:  expr.name  /  expr.name(args)
                TokenKind::Dot => {
                    let _ = self.consume();
                    let field_ident = self.consume_identifier();
                    let field = self.make_ident(&field_ident);
                    let field_span = field.span;

                    if self.peek(0).kind == TokenKind::LParen {
                        // Methodenaufruf: expr.method(arg1, arg2)
                        let (args, close) = self.parse_call_args()?;
                        let span = Span::merge(expr.span, close.span);
                        expr = ASTExpr::method_call(expr, field, args, span);
                    } else {
                        // Feldzugriff: expr.field
                        let span = Span::merge(expr.span, field_span);
                        expr = ASTExpr::field_access(expr, field, span);
                    }
                }

                // Freier Aufruf: expr(arg1, arg2)
                TokenKind::LParen => {
                    let (args, close) = self.parse_call_args()?;
                    let span = Span::merge(expr.span, close.span);
                    expr = ASTExpr::call(expr, args, span);
                }

                // Index-Zugriff: expr[idx]
                TokenKind::LBracket => {
                    self.advance(1); // '['
                    let idx = self.parse_expr()?;
                    let close = self.consume_check(TokenKind::RBracket)?;
                    let span = Span::merge(expr.span, close.span);
                    expr = ASTExpr::index(expr, idx, span);
                }

                // Pfad-Fortsetzung: expr::segment  (z.B. pkg::sub::func)
                // Nur sinnvoll wenn expr ein Pfad/Ident ist — Typchecker klärt den Rest
                TokenKind::DoubleColon => {
                    self.advance(1); // '::'
                    let ident_token = self.consume_identifier();
                    let ident = self.make_ident(&ident_token);
                    let span = Span::merge(expr.span, ident.span);
                    expr = ASTExpr::path_segment(expr, ident, span);
                }

                _ => break,
            }
        }

        Ok(expr)
    }
}