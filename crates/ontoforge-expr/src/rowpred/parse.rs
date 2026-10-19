//! CEL text to [`Pred`]: parse with `cel`, lower to the own IR, same-row and type checks.
//!
//! The `cel` program never leaves the [`analyze`](crate::analyze()) closure: the AST
//! is lowered there into an owned [`Pred`]. Evaluation never goes through `cel`.

use std::collections::BTreeMap;

use cel::common::ast::operators as op;
use cel::common::ast::{CallExpr, Expr, IdedExpr, LiteralValue, SourceInfo};
use ontoforge_schema::ir::{AttrType, BaseType, ObjectType};

use super::ir::{CmpOp, ListSource, MatchKind, Operand, Pred};
use super::value::Scalar;
use crate::analyze::{ExprError, analyze};
use crate::walk::char_pos;

/// What a row filter may refer to.
pub struct PredContext<'a> {
    /// The object type whose rows the filter selects.
    pub object_type: &'a ObjectType,
    /// The declared principal attributes (`[auth.principal_attrs]`, D-05).
    pub attrs: &'a BTreeMap<String, AttrType>,
}

/// A row filter problem, positioned inside the expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredIssue {
    pub message: String,
    /// 1-based `(line, column)` inside the expression.
    pub pos: Option<(usize, usize)>,
}

/// Compiles `src` into a [`Pred`].
///
/// # Errors
///
/// Every problem found, positioned inside the expression.
pub fn compile_row_filter(src: &str, cx: &PredContext<'_>) -> Result<Pred, Vec<PredIssue>> {
    let lowered = analyze(src, |program| {
        let mut lowerer = Lowerer {
            info: program.source_info(),
            src,
            cx,
            issues: Vec::new(),
        };
        let pred = lowerer.pred(program.expression());
        (pred, lowerer.issues)
    });
    match lowered {
        Ok((Some(pred), issues)) if issues.is_empty() => Ok(pred),
        Ok((_, issues)) if !issues.is_empty() => Err(issues),
        Ok(_) => Err(vec![PredIssue {
            message: "internal error while lowering the row filter".to_owned(),
            pos: None,
        }]),
        Err(e) => Err(vec![expr_issue(&e)]),
    }
}

fn expr_issue(e: &ExprError) -> PredIssue {
    match e {
        ExprError::Syntax {
            line,
            column,
            message,
        } => PredIssue {
            message: format!("syntax error: {message}"),
            pos: Some((*line, *column)),
        },
        other => PredIssue {
            message: other.to_string(),
            pos: None,
        },
    }
}

/// Value family of an operand. Comparisons need both sides in the same family; `int`
/// and `double` do not mix (Rust and SQL would otherwise disagree on the conversion).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fam {
    Bool,
    Int,
    Double,
    Str,
}

impl Fam {
    fn name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Double => "double",
            Self::Str => "string",
        }
    }

    fn ordered(self) -> bool {
        self != Self::Bool
    }
}

/// Static type of an operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    Fam(Fam),
    /// A property type row filters cannot compare (date, timestamp, decimal, array,
    /// geopoint). Only `== null` / `!= null` may read it.
    Opaque(BaseType),
    /// A list-typed `principal.attrs.<name>`; only valid on the right of `in`.
    List(Fam),
    Null,
}

/// Friendly name of an operator or function in an "is not allowed" message.
fn describe(func: &str) -> String {
    match func {
        op::ADD | op::SUBSTRACT | op::MULTIPLY | op::DIVIDE | op::MODULO | op::NEGATE => {
            "arithmetic".to_owned()
        }
        op::CONDITIONAL => "the conditional operator `? :`".to_owned(),
        op::INDEX | op::OPT_INDEX => "indexing".to_owned(),
        "size" | "matches" => format!("`{func}()`"),
        _ => format!("`{func}`"),
    }
}

fn property_ty(base: BaseType) -> Ty {
    match base {
        BaseType::Byte | BaseType::Short | BaseType::Integer | BaseType::Long => Ty::Fam(Fam::Int),
        BaseType::Float | BaseType::Double => Ty::Fam(Fam::Double),
        BaseType::String => Ty::Fam(Fam::Str),
        BaseType::Boolean => Ty::Fam(Fam::Bool),
        other => Ty::Opaque(other),
    }
}

fn attr_ty(ty: AttrType) -> Ty {
    match ty {
        AttrType::String => Ty::Fam(Fam::Str),
        AttrType::Integer => Ty::Fam(Fam::Int),
        AttrType::Boolean => Ty::Fam(Fam::Bool),
        AttrType::StringList => Ty::List(Fam::Str),
        AttrType::IntegerList => Ty::List(Fam::Int),
    }
}

struct Lowerer<'a> {
    info: &'a SourceInfo,
    src: &'a str,
    cx: &'a PredContext<'a>,
    issues: Vec<PredIssue>,
}

impl Lowerer<'_> {
    fn issue(&mut self, id: u64, message: impl Into<String>) {
        let pos = self
            .info
            .offset_for(id)
            .map(|(start, _)| char_pos(self.src, start as usize));
        self.issues.push(PredIssue {
            message: message.into(),
            pos,
        });
    }

    /// Lowers a node in boolean position. `None` means an issue was recorded.
    fn pred(&mut self, e: &IdedExpr) -> Option<Pred> {
        match &e.expr {
            Expr::Literal(LiteralValue::Boolean(b)) => Some(Pred::Const(*b.inner())),
            Expr::Call(call) => self.call_pred(e, call),
            Expr::Comprehension(_) => {
                self.issue(
                    e.id,
                    "macros (all, exists, exists_one, map, filter) are not allowed in a row filter",
                );
                None
            }
            Expr::Select(sel) if sel.test => {
                self.issue(e.id, "has() is not allowed in a row filter");
                None
            }
            _ => {
                self.issue(
                    e.id,
                    "a row filter must be a comparison, `in`, startsWith/endsWith/contains, \
                     or a combination of them with && || !",
                );
                None
            }
        }
    }

    fn call_pred(&mut self, e: &IdedExpr, call: &CallExpr) -> Option<Pred> {
        match (call.func_name.as_str(), call.args.as_slice()) {
            (op::LOGICAL_AND, [a, b]) => {
                let (a, b) = (self.pred(a), self.pred(b));
                Some(Pred::And(Box::new(a?), Box::new(b?)))
            }
            (op::LOGICAL_OR, [a, b]) => {
                let (a, b) = (self.pred(a), self.pred(b));
                Some(Pred::Or(Box::new(a?), Box::new(b?)))
            }
            (op::LOGICAL_NOT, [a]) => Some(Pred::Not(Box::new(self.pred(a)?))),
            (op::EQUALS, [a, b]) => self.cmp(e.id, CmpOp::Eq, a, b),
            (op::NOT_EQUALS, [a, b]) => self.cmp(e.id, CmpOp::Ne, a, b),
            (op::LESS, [a, b]) => self.cmp(e.id, CmpOp::Lt, a, b),
            (op::LESS_EQUALS, [a, b]) => self.cmp(e.id, CmpOp::Le, a, b),
            (op::GREATER, [a, b]) => self.cmp(e.id, CmpOp::Gt, a, b),
            (op::GREATER_EQUALS, [a, b]) => self.cmp(e.id, CmpOp::Ge, a, b),
            (op::IN, [x, list]) => self.in_list(e.id, x, list),
            (name, args) => {
                let kind = match name {
                    "startsWith" => Some(MatchKind::StartsWith),
                    "endsWith" => Some(MatchKind::EndsWith),
                    "contains" => Some(MatchKind::Contains),
                    _ => None,
                };
                if let (Some(kind), Some(target), [needle]) = (kind, call.target.as_deref(), args) {
                    return self.str_match(kind, target, needle);
                }
                self.issue(
                    e.id,
                    format!("{} is not allowed in a row filter", describe(name)),
                );
                None
            }
        }
    }

    fn in_list(&mut self, id: u64, x: &IdedExpr, list: &IdedExpr) -> Option<Pred> {
        let (operand, list) = (self.operand(x), self.list_source(list));
        let ((operand, ty), (list, element)) = (operand?, list?);
        let fam = match ty {
            Ty::Fam(f) => f,
            Ty::Null => {
                self.issue(id, "`null` cannot be tested with `in`; use `== null`");
                return None;
            }
            Ty::Opaque(t) => {
                self.issue(
                    id,
                    format!("a {} property cannot be used with `in`", t.as_str()),
                );
                return None;
            }
            Ty::List(_) => {
                self.issue(
                    id,
                    "a list-typed principal attribute can only be used on the right of `in`",
                );
                return None;
            }
        };
        if let Some(element) = element
            && element != fam
        {
            self.issue(
                id,
                format!(
                    "cannot test a {} value against a list of {} values",
                    fam.name(),
                    element.name()
                ),
            );
            return None;
        }
        Some(Pred::InList { operand, list })
    }

    /// The right side of `in`: a list literal of non-null literals of one family, or a
    /// list-typed principal attribute. The family is `None` for `[]`.
    fn list_source(&mut self, e: &IdedExpr) -> Option<(ListSource, Option<Fam>)> {
        if let Expr::List(list) = &e.expr {
            let mut items = Vec::with_capacity(list.elements.len());
            let mut family: Option<Fam> = None;
            let mut ok = true;
            for element in &list.elements {
                let Some((operand, ty)) = self.operand(element) else {
                    ok = false;
                    continue;
                };
                match (operand, ty) {
                    (Operand::Lit(s), Ty::Fam(f)) => {
                        if family.is_some_and(|first| first != f) {
                            self.issue(element.id, "list elements must all have the same type");
                            ok = false;
                        }
                        family.get_or_insert(f);
                        items.push(s);
                    }
                    (_, Ty::Null) => {
                        self.issue(
                            element.id,
                            "a list in a row filter must not contain `null`; test it with `== null`",
                        );
                        ok = false;
                    }
                    _ => {
                        self.issue(element.id, "list elements must be literals");
                        ok = false;
                    }
                }
            }
            return ok.then_some((ListSource::Literal(items), family));
        }
        let (operand, ty) = self.operand(e)?;
        if let (Operand::Attr(name), Ty::List(f)) = (operand, ty) {
            return Some((ListSource::Attr(name), Some(f)));
        }
        self.issue(
            e.id,
            "the right side of `in` must be a list literal or a list-typed              `principal.attrs.<name>`",
        );
        None
    }

    fn str_match(&mut self, kind: MatchKind, target: &IdedExpr, needle: &IdedExpr) -> Option<Pred> {
        let (t, n) = (self.operand(target), self.operand(needle));
        let column = match t {
            Some((Operand::Column(c), Ty::Fam(Fam::Str))) => Some(c),
            Some(_) => {
                self.issue(
                    target.id,
                    "startsWith, endsWith and contains apply to an `object.<property>` of type string",
                );
                None
            }
            None => None,
        };
        let needle = match n {
            Some((o @ (Operand::Lit(Scalar::String(_)) | Operand::Attr(_)), Ty::Fam(Fam::Str))) => {
                Some(o)
            }
            Some(_) => {
                self.issue(
                    needle.id,
                    "the argument must be a string literal or a string `principal.attrs.<name>`",
                );
                None
            }
            None => None,
        };
        Some(Pred::StrMatch {
            column: column?,
            needle: needle?,
            kind,
        })
    }

    fn cmp(&mut self, id: u64, op: CmpOp, a: &IdedExpr, b: &IdedExpr) -> Option<Pred> {
        let (left, right) = (self.operand(a), self.operand(b));
        let ((left, lt), (right, rt)) = (left?, right?);
        match (lt, rt) {
            (Ty::Null, Ty::Null) => {
                self.issue(id, "`null` cannot be compared with `null`");
                None
            }
            (Ty::Null, _) | (_, Ty::Null) => {
                let (operand, ty) = if lt == Ty::Null {
                    (right, rt)
                } else {
                    (left, lt)
                };
                self.is_null(id, op, operand, ty)
            }
            (Ty::Fam(l), Ty::Fam(r)) if l == r => {
                if op != CmpOp::Eq && op != CmpOp::Ne && !l.ordered() {
                    self.issue(id, "bool values can only be compared with == and !=");
                    return None;
                }
                Some(Pred::Cmp { left, op, right })
            }
            (Ty::Fam(l), Ty::Fam(r)) => {
                self.issue(
                    id,
                    format!(
                        "cannot compare {} with {}: no implicit conversion between value families",
                        l.name(),
                        r.name()
                    ),
                );
                None
            }
            (Ty::Opaque(t), _) | (_, Ty::Opaque(t)) => {
                self.issue(
                    id,
                    format!(
                        "a {} property cannot be used in a comparison; it can only be compared \
                         with `null`",
                        t.as_str()
                    ),
                );
                None
            }
            (Ty::List(_), _) | (_, Ty::List(_)) => {
                self.issue(
                    id,
                    "a list-typed principal attribute can only be used on the right of `in`",
                );
                None
            }
        }
    }

    fn is_null(&mut self, id: u64, op: CmpOp, operand: Operand, ty: Ty) -> Option<Pred> {
        if op != CmpOp::Eq && op != CmpOp::Ne {
            self.issue(id, "`null` can only be compared with == and !=");
            return None;
        }
        if matches!(operand, Operand::Lit(_)) {
            self.issue(
                id,
                "`== null` needs an object property or a principal attribute",
            );
            return None;
        }
        match ty {
            Ty::Opaque(BaseType::Geopoint) => {
                self.issue(id, "a geopoint property cannot be used in a row filter");
                None
            }
            Ty::List(_) => {
                self.issue(
                    id,
                    "a list-typed principal attribute can only be used on the right of `in`",
                );
                None
            }
            _ => Some(Pred::IsNull {
                operand,
                negated: op == CmpOp::Ne,
            }),
        }
    }

    /// Lowers a node in operand position.
    fn operand(&mut self, e: &IdedExpr) -> Option<(Operand, Ty)> {
        match &e.expr {
            Expr::Literal(lit) => match lit {
                LiteralValue::Boolean(b) => {
                    Some((Operand::Lit(Scalar::Bool(*b.inner())), Ty::Fam(Fam::Bool)))
                }
                LiteralValue::Int(i) => {
                    Some((Operand::Lit(Scalar::Int(*i.inner())), Ty::Fam(Fam::Int)))
                }
                LiteralValue::Double(d) => Some((
                    Operand::Lit(Scalar::Double(*d.inner())),
                    Ty::Fam(Fam::Double),
                )),
                LiteralValue::String(s) => Some((
                    Operand::Lit(Scalar::String(s.inner().to_owned())),
                    Ty::Fam(Fam::Str),
                )),
                LiteralValue::Null => Some((Operand::Lit(Scalar::Null), Ty::Null)),
                LiteralValue::Bytes(_) | LiteralValue::UInt(_) => {
                    self.issue(
                        e.id,
                        "bytes and uint literals are not allowed in a row filter",
                    );
                    None
                }
            },
            Expr::Select(sel) if sel.test => {
                self.issue(e.id, "has() is not allowed in a row filter");
                None
            }
            Expr::Select(_) => self.path(e),
            Expr::Ident(name) => {
                self.root_issue(e.id, name);
                None
            }
            Expr::Comprehension(_) => {
                self.issue(
                    e.id,
                    "macros (all, exists, exists_one, map, filter) are not allowed in a row filter",
                );
                None
            }
            Expr::Call(call) => {
                self.issue(
                    e.id,
                    format!("`{}` is not allowed in a row filter", call.func_name),
                );
                None
            }
            _ => {
                self.issue(e.id, "this expression is not allowed in a row filter");
                None
            }
        }
    }

    fn root_issue(&mut self, id: u64, name: &str) {
        let message = match name {
            "object" | "principal" => format!(
                "`{name}` cannot be used on its own; write `object.<property>` or \
                 `principal.attrs.<name>`"
            ),
            _ => format!(
                "unknown variable `{name}`: a row filter can only use `object` and \
                 `principal.attrs`"
            ),
        };
        self.issue(id, message);
    }

    /// Resolves `object.<property>` or `principal.attrs.<name>`.
    fn path(&mut self, e: &IdedExpr) -> Option<(Operand, Ty)> {
        let mut fields = Vec::new();
        let mut node = e;
        while let Expr::Select(sel) = &node.expr {
            if sel.test {
                self.issue(e.id, "has() is not allowed in a row filter");
                return None;
            }
            fields.push(sel.field.as_str());
            node = &sel.operand;
        }
        fields.reverse();
        let Expr::Ident(root) = &node.expr else {
            self.issue(
                e.id,
                "only `object.<property>` and `principal.attrs.<name>` can be read",
            );
            return None;
        };
        let root_id = node.id;
        match (root.as_str(), fields.as_slice()) {
            ("object", [name]) => self.property(root_id, name),
            ("object", _) => {
                self.issue(
                    root_id,
                    "write `object.<property>` with exactly one property name",
                );
                None
            }
            ("principal", ["attrs", name]) => self.attr(root_id, name),
            ("principal", ["id" | "groups"]) => {
                self.issue(
                    root_id,
                    "`principal.id` and `principal.groups` are not available in a row filter; \
                     use `principal.attrs.<name>`",
                );
                None
            }
            ("principal", _) => {
                self.issue(root_id, "write `principal.attrs.<name>`");
                None
            }
            (other, _) => {
                self.root_issue(root_id, other);
                None
            }
        }
    }

    fn property(&mut self, id: u64, name: &str) -> Option<(Operand, Ty)> {
        let Some((key, def)) = self.cx.object_type.properties.get_key_value(name) else {
            self.issue(
                id,
                format!(
                    "object type `{}` has no property `{name}`",
                    self.cx.object_type.api_name
                ),
            );
            return None;
        };
        Some((Operand::Column(key.clone()), property_ty(def.base_type)))
    }

    fn attr(&mut self, id: u64, name: &str) -> Option<(Operand, Ty)> {
        let Some(ty) = self.cx.attrs.get(name) else {
            self.issue(
                id,
                format!(
                    "principal attribute `{name}` is not declared in `[auth.principal_attrs]` \
                     of ontoforge-server.toml"
                ),
            );
            return None;
        };
        Some((Operand::Attr(name.to_owned()), attr_ty(*ty)))
    }
}
