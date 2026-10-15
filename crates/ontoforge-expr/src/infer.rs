//! Conservative type inference over the CEL AST (RESEARCH A7).
//!
//! `cel` has no declarative typing, so this is a small inference of our own: only
//! bool, int, double, string and lists take part in compatibility checks. Everything
//! else (decimal, date, timestamp, geopoint, object references, unknown functions,
//! unresolved references) is [`Ty::Any`], which is compatible with every type and
//! never produces an error.
//!
//! The walk recurses once per AST level: call [`type_issues`] only from inside the
//! [`analyze`](crate::analyze()) closure, which runs on the large-stack thread and
//! only ever sees prechecked input.

use std::collections::HashMap;
use std::fmt;

use cel::common::ast::operators as op;
use cel::common::ast::{CallExpr, Expr, IdedExpr, LiteralValue, SourceInfo};
use ontoforge_schema::ir::{ActionType, BaseType, Ontology, ParamType};

use crate::walk::offset_to_line_col;

/// An inferred type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Int,
    Double,
    String,
    List(Box<Ty>),
    /// Not inferable; compatible with everything.
    Any,
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Double => "double",
            Self::String => "string",
            Self::List(_) => "list",
            Self::Any => "any",
        })
    }
}

impl Ty {
    fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Double)
    }

    /// Whether values of the two types may be compared. Int and double mix.
    fn compatible(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Any, _) | (_, Self::Any) => true,
            (Self::List(a), Self::List(b)) => a.compatible(b),
            (a, b) => a == b || (a.is_numeric() && b.is_numeric()),
        }
    }
}

fn base_ty(base: BaseType, items: Option<BaseType>) -> Ty {
    match base {
        BaseType::Byte | BaseType::Short | BaseType::Integer | BaseType::Long => Ty::Int,
        BaseType::Float | BaseType::Double => Ty::Double,
        BaseType::String => Ty::String,
        BaseType::Boolean => Ty::Bool,
        BaseType::Array => Ty::List(Box::new(items.map_or(Ty::Any, |i| base_ty(i, None)))),
        BaseType::Decimal | BaseType::Date | BaseType::Timestamp | BaseType::Geopoint => Ty::Any,
    }
}

/// The types of the variables one action's conditions can see.
#[derive(Debug, Default)]
pub struct TypeEnv {
    params: HashMap<String, Ty>,
    /// `objectReference` parameter name to the property types of its object type.
    objects: HashMap<String, HashMap<String, Ty>>,
}

impl TypeEnv {
    #[must_use]
    pub fn for_action(action: &ActionType, ontology: &Ontology) -> Self {
        let mut env = Self::default();
        for (name, def) in &action.parameters {
            let ty = def
                .param_type
                .base_type()
                .map_or(Ty::Any, |base| base_ty(base, def.items));
            env.params.insert(name.as_str().to_owned(), ty);
            if let ParamType::ObjectReference(target) = &def.param_type
                && let Some(object_type) = ontology.object_types.get(target.as_str())
            {
                let props = object_type
                    .properties
                    .iter()
                    .map(|(prop, p)| (prop.as_str().to_owned(), base_ty(p.base_type, p.items)))
                    .collect();
                env.objects.insert(name.as_str().to_owned(), props);
            }
        }
        env
    }

    /// Type of a member path rooted at a free variable; unknown paths are `Any`.
    fn lookup(&self, segments: &[&str]) -> Ty {
        match segments {
            ["params", name] => self.params.get(*name).cloned(),
            ["objects", name, prop] => self.objects.get(*name).and_then(|p| p.get(*prop)).cloned(),
            ["user", "id"] => Some(Ty::String),
            ["user", "groups"] => Some(Ty::List(Box::new(Ty::String))),
            _ => None,
        }
        .unwrap_or(Ty::Any)
    }
}

/// A type problem, positioned inside the expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeIssue {
    pub message: String,
    /// 1-based `(line, column)` inside the expression.
    pub pos: Option<(usize, usize)>,
}

/// Incompatible comparisons, and a top-level result that is inferably not bool.
/// `src` must be the text `program` was compiled from.
#[must_use]
pub fn type_issues(program: &cel::Program, src: &str, env: &TypeEnv) -> Vec<TypeIssue> {
    let mut inferer = Inferer {
        info: program.source_info(),
        src,
        env,
        scope: Vec::new(),
        issues: Vec::new(),
    };
    let root = program.expression();
    let ty = inferer.infer(root);
    if !matches!(ty, Ty::Bool | Ty::Any) {
        inferer.issue(
            root.id,
            format!("condition must evaluate to bool, found {ty}"),
        );
    }
    inferer.issues
}

struct Inferer<'a> {
    info: &'a SourceInfo,
    src: &'a str,
    env: &'a TypeEnv,
    /// Comprehension variables currently in scope, innermost last.
    scope: Vec<(&'a str, Ty)>,
    issues: Vec<TypeIssue>,
}

impl<'a> Inferer<'a> {
    fn issue(&mut self, id: u64, message: String) {
        let pos = self
            .info
            .offset_for(id)
            .map(|(start, _)| offset_to_line_col(self.src, start as usize));
        self.issues.push(TypeIssue { message, pos });
    }

    fn scoped(&self, name: &str) -> Option<&Ty> {
        self.scope
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map(|(_, t)| t)
    }

    fn infer(&mut self, e: &'a IdedExpr) -> Ty {
        match &e.expr {
            Expr::Unspecified => Ty::Any,
            Expr::Literal(lit) => match lit {
                LiteralValue::Boolean(_) => Ty::Bool,
                LiteralValue::Int(_) => Ty::Int,
                LiteralValue::Double(_) => Ty::Double,
                LiteralValue::String(_) => Ty::String,
                LiteralValue::Bytes(_) | LiteralValue::UInt(_) | LiteralValue::Null => Ty::Any,
            },
            Expr::Ident(name) => self.scoped(name).cloned().unwrap_or(Ty::Any),
            Expr::Select(_) => self.select(e),
            Expr::Call(call) => self.call(e.id, call),
            Expr::List(list) => {
                let tys: Vec<Ty> = list.elements.iter().map(|el| self.infer(el)).collect();
                let element = match tys.split_first() {
                    Some((first, rest)) if rest.iter().all(|t| t == first) => first.clone(),
                    _ => Ty::Any,
                };
                Ty::List(Box::new(element))
            }
            Expr::Map(map) => {
                self.entries(&map.entries);
                Ty::Any
            }
            Expr::Struct(s) => {
                self.entries(&s.entries);
                Ty::Any
            }
            Expr::Comprehension(c) => {
                let range = self.infer(&c.iter_range);
                let accu = self.infer(&c.accu_init);
                let element = match range {
                    Ty::List(inner) => *inner,
                    _ => Ty::Any,
                };
                let mark = self.scope.len();
                self.scope.push((&c.iter_var, element));
                if let Some(second) = &c.iter_var2 {
                    self.scope.push((second, Ty::Any));
                }
                self.scope.push((&c.accu_var, accu));
                self.infer(&c.loop_cond);
                self.infer(&c.loop_step);
                let result = self.infer(&c.result);
                self.scope.truncate(mark);
                result
            }
        }
    }

    fn entries(&mut self, entries: &'a [cel::common::ast::IdedEntryExpr]) {
        use cel::common::ast::EntryExpr;
        for entry in entries {
            match &entry.expr {
                EntryExpr::MapEntry(m) => {
                    self.infer(&m.key);
                    self.infer(&m.value);
                }
                EntryExpr::StructField(f) => {
                    self.infer(&f.value);
                }
            }
        }
    }

    /// A field selection: a member path rooted at a free variable is looked up in the
    /// environment; any other shape is walked for its comparisons and is `Any`.
    fn select(&mut self, e: &'a IdedExpr) -> Ty {
        let mut fields = Vec::new();
        let mut node = e;
        while let Expr::Select(sel) = &node.expr {
            if sel.test {
                // A `has()` test is a bool and ends the path.
                if std::ptr::eq(node, e) {
                    self.infer(&sel.operand);
                    return Ty::Bool;
                }
                break;
            }
            fields.push(sel.field.as_str());
            node = &sel.operand;
        }
        match &node.expr {
            Expr::Ident(root) if self.scoped(root).is_none() => {
                let segments: Vec<&str> = std::iter::once(root.as_str())
                    .chain(fields.iter().rev().copied())
                    .collect();
                self.env.lookup(&segments)
            }
            _ => {
                self.infer(node);
                Ty::Any
            }
        }
    }

    fn call(&mut self, id: u64, call: &'a CallExpr) -> Ty {
        if let Some(target) = &call.target {
            self.infer(target);
        }
        let args: Vec<Ty> = call.args.iter().map(|a| self.infer(a)).collect();
        match call.func_name.as_str() {
            op::LOGICAL_AND
            | op::LOGICAL_OR
            | op::LOGICAL_NOT
            | op::IN
            | op::NOT_STRICTLY_FALSE => Ty::Bool,
            op::EQUALS
            | op::NOT_EQUALS
            | op::LESS
            | op::LESS_EQUALS
            | op::GREATER
            | op::GREATER_EQUALS => {
                if let [left, right] = args.as_slice()
                    && !left.compatible(right)
                {
                    self.issue(id, format!("cannot compare {left} with {right}"));
                }
                Ty::Bool
            }
            op::ADD => match args.as_slice() {
                [Ty::String, Ty::String] => Ty::String,
                [left, right] => arithmetic(left, right),
                _ => Ty::Any,
            },
            op::SUBSTRACT | op::MULTIPLY | op::DIVIDE | op::MODULO => match args.as_slice() {
                [left, right] => arithmetic(left, right),
                _ => Ty::Any,
            },
            op::NEGATE => match args.as_slice() {
                [only] if only.is_numeric() => only.clone(),
                _ => Ty::Any,
            },
            op::CONDITIONAL => match args.as_slice() {
                [_, then, otherwise] if then == otherwise => then.clone(),
                _ => Ty::Any,
            },
            "size" => Ty::Int,
            op::HAS | "matches" | "startsWith" | "endsWith" | "contains" => Ty::Bool,
            _ => Ty::Any,
        }
    }
}

fn arithmetic(left: &Ty, right: &Ty) -> Ty {
    match (left, right) {
        (Ty::Int, Ty::Int) => Ty::Int,
        (l, r) if l.is_numeric() && r.is_numeric() => Ty::Double,
        _ => Ty::Any,
    }
}
