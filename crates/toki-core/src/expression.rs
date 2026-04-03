use crate::value_path::{
    ResolvedValue, ResolvedValueRef, ValuePath, ValuePathContext, ValuePathError,
};
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    root: ExprNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExprNode {
    Literal(ResolvedValue),
    Path(ValuePath),
    Unary {
        op: UnaryOp,
        expr: Box<ExprNode>,
    },
    Binary {
        left: Box<ExprNode>,
        op: BinaryOp,
        right: Box<ExprNode>,
    },
    Call {
        name: String,
        args: Vec<ExprNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOp {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExpressionError {
    #[error("parse error at {span:?}: {message}")]
    Parse { span: TextSpan, message: String },
    #[error("value path error: {0}")]
    ValuePath(#[from] ValuePathError),
    #[error("evaluation error at {span:?}: {message}")]
    Evaluation { span: TextSpan, message: String },
}

impl Expression {
    pub fn parse(source: &str) -> Result<Self, ExpressionError> {
        let mut parser = Parser::new(source)?;
        let root = parser.parse_expression()?;
        parser.expect_end()?;
        Ok(Self { root })
    }

    pub fn evaluate(
        &self,
        context: ValuePathContext<'_, '_>,
    ) -> Result<ResolvedValue, ExpressionError> {
        evaluate_node(&self.root, context).map(EvalValue::into_owned)
    }
}

type EvalValue<'a> = ResolvedValueRef<'a>;

fn evaluate_node<'a>(
    node: &'a ExprNode,
    context: ValuePathContext<'a, '_>,
) -> Result<EvalValue<'a>, ExpressionError> {
    match node {
        ExprNode::Literal(value) => Ok(borrow_resolved_value(value)),
        ExprNode::Path(path) => path.resolve_borrowed(context).map_err(ExpressionError::from),
        ExprNode::Unary { op, expr } => {
            let value = evaluate_node(expr, context)?;
            match (op, value) {
                (UnaryOp::Not, EvalValue::Bool(value)) => Ok(EvalValue::Bool(!value)),
                (UnaryOp::Negate, EvalValue::Int(value)) => {
                    Ok(EvalValue::Int(value.saturating_neg()))
                }
                (UnaryOp::Not, value) => Err(eval_error(format!(
                    "logical not requires a bool, got {:?}",
                    value
                ))),
                (UnaryOp::Negate, value) => Err(eval_error(format!(
                    "negation requires an int, got {:?}",
                    value
                ))),
            }
        }
        ExprNode::Binary { left, op, right } => {
            let left = evaluate_node(left, context)?;
            let right = evaluate_node(right, context)?;
            evaluate_binary(*op, left, right)
        }
        ExprNode::Call { name, args } => evaluate_call(name, args, context),
    }
}

fn evaluate_binary(
    op: BinaryOp,
    left: EvalValue<'_>,
    right: EvalValue<'_>,
) -> Result<EvalValue<'static>, ExpressionError> {
    match op {
        BinaryOp::Add => int_bin_op(left, right, i32::saturating_add),
        BinaryOp::Subtract => int_bin_op(left, right, i32::saturating_sub),
        BinaryOp::Multiply => int_bin_op(left, right, i32::saturating_mul),
        BinaryOp::Divide => {
            let (left, right) = expect_int_pair(left, right, "division")?;
            if right == 0 {
                return Err(eval_error("division by zero"));
            }
            Ok(EvalValue::Int(left / right))
        }
        BinaryOp::Equal => Ok(EvalValue::Bool(left == right)),
        BinaryOp::NotEqual => Ok(EvalValue::Bool(left != right)),
        BinaryOp::Less => cmp_ints(left, right, |l, r| l < r),
        BinaryOp::LessOrEqual => cmp_ints(left, right, |l, r| l <= r),
        BinaryOp::Greater => cmp_ints(left, right, |l, r| l > r),
        BinaryOp::GreaterOrEqual => cmp_ints(left, right, |l, r| l >= r),
        BinaryOp::And => {
            let (left, right) = expect_bool_pair(left, right, "logical and")?;
            Ok(EvalValue::Bool(left && right))
        }
        BinaryOp::Or => {
            let (left, right) = expect_bool_pair(left, right, "logical or")?;
            Ok(EvalValue::Bool(left || right))
        }
    }
}

fn evaluate_call<'a>(
    name: &str,
    args: &'a [ExprNode],
    context: ValuePathContext<'a, '_>,
) -> Result<EvalValue<'static>, ExpressionError> {
    match name {
        "min" => {
            let values = eval_int_args(args, context, 2, "min")?;
            Ok(EvalValue::Int(values[0].min(values[1])))
        }
        "max" => {
            let values = eval_int_args(args, context, 2, "max")?;
            Ok(EvalValue::Int(values[0].max(values[1])))
        }
        "abs" => {
            let values = eval_int_args(args, context, 1, "abs")?;
            Ok(EvalValue::Int(values[0].abs()))
        }
        "random" => {
            let values = eval_int_args(args, context, 2, "random")?;
            let lo = values[0];
            let hi = values[1];
            if hi < lo {
                return Err(eval_error("random(lo, hi) requires hi >= lo"));
            }
            Ok(EvalValue::Int(fastrand::i32(lo..=hi)))
        }
        other => Err(eval_error(format!("unknown function '{other}'"))),
    }
}

fn eval_int_args<'a>(
    args: &'a [ExprNode],
    context: ValuePathContext<'a, '_>,
    expected_len: usize,
    fn_name: &str,
) -> Result<Vec<i32>, ExpressionError> {
    if args.len() != expected_len {
        return Err(eval_error(format!(
            "{fn_name} expects {expected_len} argument(s), got {}",
            args.len()
        )));
    }
    args.iter()
        .map(|arg| match evaluate_node(arg, context)? {
            EvalValue::Int(value) => Ok(value),
            value => Err(eval_error(format!(
                "{fn_name} expects int arguments, got {:?}",
                value
            ))),
        })
        .collect()
}

fn int_bin_op(
    left: EvalValue<'_>,
    right: EvalValue<'_>,
    op: impl FnOnce(i32, i32) -> i32,
) -> Result<EvalValue<'static>, ExpressionError> {
    let (left, right) = expect_int_pair(left, right, "integer operation")?;
    Ok(EvalValue::Int(op(left, right)))
}

fn cmp_ints(
    left: EvalValue<'_>,
    right: EvalValue<'_>,
    cmp: impl FnOnce(i32, i32) -> bool,
) -> Result<EvalValue<'static>, ExpressionError> {
    let (left, right) = expect_int_pair(left, right, "comparison")?;
    Ok(EvalValue::Bool(cmp(left, right)))
}

fn expect_int_pair(
    left: EvalValue<'_>,
    right: EvalValue<'_>,
    op_name: &str,
) -> Result<(i32, i32), ExpressionError> {
    match (left, right) {
        (EvalValue::Int(left), EvalValue::Int(right)) => Ok((left, right)),
        (left, right) => Err(eval_error(format!(
            "{op_name} requires ints, got {:?} and {:?}",
            left, right
        ))),
    }
}

fn expect_bool_pair(
    left: EvalValue<'_>,
    right: EvalValue<'_>,
    op_name: &str,
) -> Result<(bool, bool), ExpressionError> {
    match (left, right) {
        (EvalValue::Bool(left), EvalValue::Bool(right)) => Ok((left, right)),
        (left, right) => Err(eval_error(format!(
            "{op_name} requires bools, got {:?} and {:?}",
            left, right
        ))),
    }
}

fn eval_error(message: impl Into<String>) -> ExpressionError {
    ExpressionError::Evaluation {
        span: TextSpan { start: 0, end: 0 },
        message: message.into(),
    }
}

fn borrow_resolved_value(value: &ResolvedValue) -> EvalValue<'_> {
    match value {
        ResolvedValue::Bool(value) => EvalValue::Bool(*value),
        ResolvedValue::Int(value) => EvalValue::Int(*value),
        ResolvedValue::String(value) => EvalValue::String(Cow::Borrowed(value.as_str())),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Int(i32),
    Bool(bool),
    String(String),
    Ident(String),
    LeftParen,
    RightParen,
    Comma,
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    AndAnd,
    OrOr,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    span: TextSpan,
}

struct Lexer<'a> {
    source: &'a str,
    chars: std::str::CharIndices<'a>,
    peeked: Option<(usize, char)>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices(),
            peeked: None,
        }
    }

    fn next_token(&mut self) -> Result<Token, ExpressionError> {
        self.skip_whitespace();
        let Some((start, ch)) = self.next_char() else {
            let end = self.source.len();
            return Ok(Token {
                kind: TokenKind::End,
                span: TextSpan { start: end, end },
            });
        };

        let token = match ch {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            ',' => TokenKind::Comma,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '!' => {
                if self.consume_if('=') {
                    TokenKind::BangEqual
                } else {
                    TokenKind::Bang
                }
            }
            '=' => {
                if self.consume_if('=') {
                    TokenKind::EqualEqual
                } else {
                    return Err(ExpressionError::Parse {
                        span: TextSpan {
                            start,
                            end: start + 1,
                        },
                        message: "expected '=' after '='".to_string(),
                    });
                }
            }
            '<' => {
                if self.consume_if('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                if self.consume_if('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            '&' => {
                if self.consume_if('&') {
                    TokenKind::AndAnd
                } else {
                    return Err(ExpressionError::Parse {
                        span: TextSpan {
                            start,
                            end: start + 1,
                        },
                        message: "expected '&' after '&'".to_string(),
                    });
                }
            }
            '|' => {
                if self.consume_if('|') {
                    TokenKind::OrOr
                } else {
                    return Err(ExpressionError::Parse {
                        span: TextSpan {
                            start,
                            end: start + 1,
                        },
                        message: "expected '|' after '|'".to_string(),
                    });
                }
            }
            '"' => {
                let string = self.read_string(start)?;
                return Ok(Token {
                    kind: TokenKind::String(string),
                    span: TextSpan {
                        start,
                        end: self.current_index(),
                    },
                });
            }
            ch if ch.is_ascii_digit() => {
                let value = self.read_number(start, ch)?;
                return Ok(Token {
                    kind: TokenKind::Int(value),
                    span: TextSpan {
                        start,
                        end: self.current_index(),
                    },
                });
            }
            ch if is_ident_start(ch) => {
                let ident = self.read_ident(start, ch)?;
                let kind = match ident.as_str() {
                    "true" => TokenKind::Bool(true),
                    "false" => TokenKind::Bool(false),
                    _ => TokenKind::Ident(ident),
                };
                return Ok(Token {
                    kind,
                    span: TextSpan {
                        start,
                        end: self.current_index(),
                    },
                });
            }
            _ => {
                return Err(ExpressionError::Parse {
                    span: TextSpan {
                        start,
                        end: start + ch.len_utf8(),
                    },
                    message: format!("unexpected character '{ch}'"),
                });
            }
        };

        Ok(Token {
            kind: token,
            span: TextSpan {
                start,
                end: self.current_index(),
            },
        })
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char().is_some_and(|(_, ch)| ch.is_whitespace()) {
            let _ = self.next_char();
        }
    }

    fn read_string(&mut self, start: usize) -> Result<String, ExpressionError> {
        let mut result = String::new();
        while let Some((_, ch)) = self.next_char() {
            match ch {
                '"' => return Ok(result),
                '\\' => {
                    let Some((_, escaped)) = self.next_char() else {
                        break;
                    };
                    result.push(match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                }
                other => result.push(other),
            }
        }
        Err(ExpressionError::Parse {
            span: TextSpan {
                start,
                end: self.current_index(),
            },
            message: "unterminated string literal".to_string(),
        })
    }

    fn read_number(&mut self, start: usize, first: char) -> Result<i32, ExpressionError> {
        let mut number = String::from(first);
        while self.peek_char().is_some_and(|(_, ch)| ch.is_ascii_digit()) {
            number.push(self.next_expected_char(start, "unterminated integer literal")?);
        }
        number
            .parse::<i32>()
            .map_err(|error| ExpressionError::Parse {
                span: TextSpan {
                    start,
                    end: self.current_index(),
                },
                message: format!("invalid integer literal: {error}"),
            })
    }

    fn read_ident(&mut self, start: usize, first: char) -> Result<String, ExpressionError> {
        let mut ident = String::from(first);
        while self
            .peek_char()
            .is_some_and(|(_, ch)| is_ident_continue(ch))
        {
            ident.push(self.next_expected_char(start, "unterminated identifier")?);
        }
        Ok(ident)
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek_char().is_some_and(|(_, ch)| ch == expected) {
            let _ = self.next_char();
            true
        } else {
            false
        }
    }

    fn peek_char(&mut self) -> Option<(usize, char)> {
        if self.peeked.is_none() {
            self.peeked = self.chars.next();
        }
        self.peeked
    }

    fn next_char(&mut self) -> Option<(usize, char)> {
        self.peeked.take().or_else(|| self.chars.next())
    }

    fn next_expected_char(&mut self, start: usize, message: &str) -> Result<char, ExpressionError> {
        self.next_char()
            .map(|(_, ch)| ch)
            .ok_or_else(|| ExpressionError::Parse {
                span: TextSpan {
                    start,
                    end: self.current_index(),
                },
                message: message.to_string(),
            })
    }

    fn current_index(&mut self) -> usize {
        self.peek_char()
            .map(|(index, _)| index)
            .unwrap_or(self.source.len())
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Result<Self, ExpressionError> {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token()?;
        Ok(Self { lexer, current })
    }

    fn parse_expression(&mut self) -> Result<ExprNode, ExpressionError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut node = self.parse_and()?;
        while matches!(self.current.kind, TokenKind::OrOr) {
            self.advance()?;
            let right = self.parse_and()?;
            node = ExprNode::Binary {
                left: Box::new(node),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(node)
    }

    fn parse_and(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut node = self.parse_equality()?;
        while matches!(self.current.kind, TokenKind::AndAnd) {
            self.advance()?;
            let right = self.parse_equality()?;
            node = ExprNode::Binary {
                left: Box::new(node),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(node)
    }

    fn parse_equality(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut node = self.parse_comparison()?;
        loop {
            let op = match self.current.kind {
                TokenKind::EqualEqual => BinaryOp::Equal,
                TokenKind::BangEqual => BinaryOp::NotEqual,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_comparison()?;
            node = ExprNode::Binary {
                left: Box::new(node),
                op,
                right: Box::new(right),
            };
        }
        Ok(node)
    }

    fn parse_comparison(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut node = self.parse_term()?;
        loop {
            let op = match self.current.kind {
                TokenKind::Less => BinaryOp::Less,
                TokenKind::LessEqual => BinaryOp::LessOrEqual,
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::GreaterEqual => BinaryOp::GreaterOrEqual,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_term()?;
            node = ExprNode::Binary {
                left: Box::new(node),
                op,
                right: Box::new(right),
            };
        }
        Ok(node)
    }

    fn parse_term(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut node = self.parse_factor()?;
        loop {
            let op = match self.current.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_factor()?;
            node = ExprNode::Binary {
                left: Box::new(node),
                op,
                right: Box::new(right),
            };
        }
        Ok(node)
    }

    fn parse_factor(&mut self) -> Result<ExprNode, ExpressionError> {
        let mut node = self.parse_unary()?;
        loop {
            let op = match self.current.kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                _ => break,
            };
            self.advance()?;
            let right = self.parse_unary()?;
            node = ExprNode::Binary {
                left: Box::new(node),
                op,
                right: Box::new(right),
            };
        }
        Ok(node)
    }

    fn parse_unary(&mut self) -> Result<ExprNode, ExpressionError> {
        match self.current.kind {
            TokenKind::Bang => {
                self.advance()?;
                Ok(ExprNode::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            TokenKind::Minus => {
                self.advance()?;
                Ok(ExprNode::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<ExprNode, ExpressionError> {
        match &self.current.kind {
            TokenKind::Int(value) => {
                let node = ExprNode::Literal(ResolvedValue::Int(*value));
                self.advance()?;
                Ok(node)
            }
            TokenKind::Bool(value) => {
                let node = ExprNode::Literal(ResolvedValue::Bool(*value));
                self.advance()?;
                Ok(node)
            }
            TokenKind::String(value) => {
                let node = ExprNode::Literal(ResolvedValue::String(value.clone()));
                self.advance()?;
                Ok(node)
            }
            TokenKind::Ident(ident) => {
                let ident = ident.clone();
                self.advance()?;
                if matches!(self.current.kind, TokenKind::LeftParen) {
                    self.advance()?;
                    let mut args = Vec::new();
                    if !matches!(self.current.kind, TokenKind::RightParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.current.kind, TokenKind::Comma) {
                                self.advance()?;
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RightParen, "expected ')' after function call")?;
                    Ok(ExprNode::Call { name: ident, args })
                } else {
                    Ok(ExprNode::Path(ValuePath::parse(&ident)?))
                }
            }
            TokenKind::LeftParen => {
                self.advance()?;
                let node = self.parse_expression()?;
                self.expect(TokenKind::RightParen, "expected ')' after expression")?;
                Ok(node)
            }
            _ => Err(ExpressionError::Parse {
                span: self.current.span,
                message: "expected expression".to_string(),
            }),
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Result<(), ExpressionError> {
        if std::mem::discriminant(&self.current.kind) == std::mem::discriminant(&kind) {
            self.advance()
        } else {
            Err(ExpressionError::Parse {
                span: self.current.span,
                message: message.to_string(),
            })
        }
    }

    fn expect_end(&mut self) -> Result<(), ExpressionError> {
        if matches!(self.current.kind, TokenKind::End) {
            Ok(())
        } else {
            Err(ExpressionError::Parse {
                span: self.current.span,
                message: "unexpected trailing tokens".to_string(),
            })
        }
    }

    fn advance(&mut self) -> Result<(), ExpressionError> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Expression;
    use crate::entity::HEALTH_STAT_ID;
    use crate::flags::{FlagValue, GameFlags};
    use crate::value_path::{ResolvedValue, ValuePathContext};
    use crate::{game::SceneSystem, rules::TriggerContext, GameState};
    use glam::IVec2;

    fn empty_context<'a>(flags: &'a GameFlags) -> ValuePathContext<'a, 'static> {
        let state = Box::leak(Box::new(GameState::new_empty()));
        let trigger = Box::leak(Box::new(TriggerContext::empty()));
        ValuePathContext {
            entity_manager: state.world().entity_manager(),
            game_flags: flags,
            player_id: None,
            trigger_context: trigger,
        }
    }

    #[test]
    fn arithmetic_respects_precedence() {
        let expr = Expression::parse("1 + 2 * 3").expect("expression should parse");
        let flags = GameFlags::default();
        assert_eq!(
            expr.evaluate(empty_context(&flags))
                .expect("expression should evaluate"),
            ResolvedValue::Int(7)
        );
    }

    #[test]
    fn comparisons_and_logical_ops_work() {
        let expr = Expression::parse("1 + 2 == 3 && !false").expect("expression should parse");
        let flags = GameFlags::default();
        assert_eq!(
            expr.evaluate(empty_context(&flags))
                .expect("expression should evaluate"),
            ResolvedValue::Bool(true)
        );
    }

    #[test]
    fn value_paths_resolve_in_expressions() {
        let mut state = GameState::new_empty();
        let player_id = SceneSystem::spawn_player_at(&mut state, IVec2::new(0, 0));
        state
            .world_mut()
            .entity_manager_mut()
            .combat_mut(player_id)
            .expect("player should exist")
            .stats
            .current
            .insert(HEALTH_STAT_ID.to_string(), 14);
        let mut flags = GameFlags::default();
        flags.set("coins", FlagValue::Int(3));
        let expr =
            Expression::parse("player.health + flags.coins").expect("expression should parse");
        let trigger = TriggerContext::with_self_only(player_id);
        let context = ValuePathContext {
            entity_manager: state.world().entity_manager(),
            game_flags: &flags,
            player_id: Some(player_id),
            trigger_context: &trigger,
        };
        assert_eq!(
            expr.evaluate(context).expect("expression should evaluate"),
            ResolvedValue::Int(17)
        );
    }

    #[test]
    fn string_value_paths_compare_without_losing_semantics() {
        let mut flags = GameFlags::default();
        flags.set("region", FlagValue::String("forest".to_string()));
        let expr =
            Expression::parse("flags.region == \"forest\"").expect("expression should parse");
        assert_eq!(
            expr.evaluate(empty_context(&flags))
                .expect("expression should evaluate"),
            ResolvedValue::Bool(true)
        );
    }

    #[test]
    fn functions_work() {
        let expr = Expression::parse("max(2, abs(-5))").expect("expression should parse");
        let flags = GameFlags::default();
        assert_eq!(
            expr.evaluate(empty_context(&flags))
                .expect("expression should evaluate"),
            ResolvedValue::Int(5)
        );
    }

    #[test]
    fn random_is_deterministic_with_seed() {
        fastrand::seed(7);
        let expr = Expression::parse("random(1, 3)").expect("expression should parse");
        let flags = GameFlags::default();
        let first = expr
            .evaluate(empty_context(&flags))
            .expect("expression should evaluate");
        fastrand::seed(7);
        let second = expr
            .evaluate(empty_context(&flags))
            .expect("expression should evaluate");
        assert_eq!(first, second);
        assert!(matches!(first, ResolvedValue::Int(value) if (1..=3).contains(&value)));
    }

    #[test]
    fn invalid_parse_reports_error() {
        let error = Expression::parse("1 +").expect_err("invalid expression should fail");
        assert!(matches!(error, super::ExpressionError::Parse { .. }));
    }

    #[test]
    fn lexer_next_expected_char_returns_parse_error_instead_of_panicking() {
        let mut lexer = super::Lexer::new("");
        let error = lexer
            .next_expected_char(0, "unterminated identifier")
            .expect_err("missing char should become a parse error");
        assert!(matches!(
            error,
            super::ExpressionError::Parse { ref message, .. }
            if message == "unterminated identifier"
        ));
    }

    #[test]
    fn type_mismatch_reports_error() {
        let expr = Expression::parse("1 + true").expect("expression should parse");
        let flags = GameFlags::default();
        let error = expr
            .evaluate(empty_context(&flags))
            .expect_err("invalid operation should fail");
        assert!(matches!(error, super::ExpressionError::Evaluation { .. }));
    }
}
