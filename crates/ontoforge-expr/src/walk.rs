//! Scope-aware walk over the CEL AST that collects member paths.
//!
//! `cel`'s own reference listing counts comprehension variables as free variables, so
//! the walk keeps its own scope stack. It recurses once per AST level: call it only
//! from inside the [`analyze`](crate::analyze()) closure, which runs on the large-stack
//! thread and only ever sees prechecked input.

use cel::common::ast::operators as op;
use cel::common::ast::{EntryExpr, Expr, IdedExpr, LiteralValue, SourceInfo};
use ontoforge_schema::diag::offset_to_line_col;

/// A member path rooted at a free variable, e.g. `params.items`. A string-literal
/// index is a segment like a field (`params['items']` is `params.items`, WR-02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberPath {
    pub segments: Vec<String>,
    /// 1-based `(line, column)` inside the expression, column counted in chars.
    pub pos: Option<(usize, usize)>,
    /// The path is followed by an index that is not a string literal, so whatever
    /// it selects next cannot be checked.
    pub dynamic: bool,
}

/// One step of a member path: `.field`, `['field']` (`Some`), or an index that is
/// not a string literal (`None`), with the operand it applies to.
fn step(e: &IdedExpr) -> Option<(Option<&str>, &IdedExpr)> {
    match &e.expr {
        Expr::Select(sel) => Some((Some(sel.field.as_str()), &sel.operand)),
        Expr::Call(call) if call.func_name == op::INDEX && call.target.is_none() => {
            match call.args.as_slice() {
                [operand, key] => match &key.expr {
                    Expr::Literal(LiteralValue::String(s)) => Some((Some(&**s), operand)),
                    _ => Some((None, operand)),
                },
                _ => None,
            }
        }
        _ => None,
    }
}

/// Collects the member paths rooted at free variables of `program`, in source walk
/// order and without deduplication. `src` must be the text `program` was compiled from.
#[must_use]
pub fn member_paths(program: &cel::Program, src: &str) -> Vec<MemberPath> {
    let mut walker = Walker {
        info: program.source_info(),
        src,
        scope: Vec::new(),
        out: Vec::new(),
    };
    walker.visit(program.expression());
    walker.out
}

/// 1-based `(line, column)` of a `cel` source offset, column counted in chars.
/// `SourceInfo::offset_for` yields 0-based UTF-8 byte offsets (verified against 0.15.0
/// with a multi-byte literal), which is what `offset_to_line_col` takes.
pub(crate) fn char_pos(src: &str, offset: usize) -> (usize, usize) {
    let (line, column) = offset_to_line_col(src, offset);
    (line as usize, column as usize)
}

struct Walker<'a> {
    info: &'a SourceInfo,
    src: &'a str,
    /// Comprehension variables currently in scope (iteration and accumulator).
    scope: Vec<&'a str>,
    out: Vec<MemberPath>,
}

impl<'a> Walker<'a> {
    fn pos(&self, ids: &[u64]) -> Option<(usize, usize)> {
        ids.iter().find_map(|&id| {
            let (start, _) = self.info.offset_for(id)?;
            Some(char_pos(self.src, start as usize))
        })
    }

    fn in_scope(&self, name: &str) -> bool {
        self.scope.contains(&name)
    }

    fn visit(&mut self, e: &'a IdedExpr) {
        match &e.expr {
            Expr::Unspecified | Expr::Literal(_) => {}
            Expr::Ident(name) => {
                if !self.in_scope(name) {
                    let pos = self.pos(&[e.id]);
                    self.out.push(MemberPath {
                        segments: vec![name.clone()],
                        pos,
                        dynamic: false,
                    });
                }
            }
            Expr::Select(_) => self.select(e),
            Expr::Call(_) if step(e).is_some() => self.select(e),
            Expr::Call(call) => {
                if let Some(target) = &call.target {
                    self.visit(target);
                }
                call.args.iter().for_each(|a| self.visit(a));
            }
            Expr::List(list) => list.elements.iter().for_each(|a| self.visit(a)),
            Expr::Map(map) => map.entries.iter().for_each(|entry| match &entry.expr {
                EntryExpr::MapEntry(m) => {
                    self.visit(&m.key);
                    self.visit(&m.value);
                }
                EntryExpr::StructField(f) => self.visit(&f.value),
            }),
            Expr::Struct(s) => s.entries.iter().for_each(|entry| match &entry.expr {
                EntryExpr::StructField(f) => self.visit(&f.value),
                EntryExpr::MapEntry(m) => {
                    self.visit(&m.key);
                    self.visit(&m.value);
                }
            }),
            Expr::Comprehension(c) => {
                self.visit(&c.iter_range);
                self.visit(&c.accu_init);
                let mark = self.scope.len();
                self.scope.push(&c.iter_var);
                if let Some(second) = &c.iter_var2 {
                    self.scope.push(second);
                }
                self.scope.push(&c.accu_var);
                self.visit(&c.loop_cond);
                self.visit(&c.loop_step);
                self.visit(&c.result);
                self.scope.truncate(mark);
            }
        }
    }

    /// Handles a chain of field selections and indexes: descends through them (and
    /// the outermost `has()` test) to its root. A free identifier root yields the
    /// path; any other root is walked on its own. A non-literal index is walked on
    /// its own and cuts the path there: only the part below it can be checked.
    fn select(&mut self, outermost: &'a IdedExpr) {
        let mut fields = Vec::new();
        let mut dynamic = false;
        let mut node = outermost;
        while let Some((field, operand)) = step(node) {
            if let Some(field) = field {
                fields.push(field);
            } else {
                if let Expr::Call(call) = &node.expr {
                    call.args.iter().skip(1).for_each(|key| self.visit(key));
                }
                fields.clear();
                dynamic = true;
            }
            node = operand;
            if matches!(&node.expr, Expr::Select(inner) if inner.test) {
                break;
            }
        }
        match &node.expr {
            Expr::Ident(root) if !self.in_scope(root) => {
                let pos = self.pos(&[node.id, outermost.id]);
                let segments = std::iter::once(root.as_str())
                    .chain(fields.iter().rev().copied())
                    .map(str::to_owned)
                    .collect();
                self.out.push(MemberPath {
                    segments,
                    pos,
                    dynamic,
                });
            }
            Expr::Ident(_) => {}
            _ => self.visit(node),
        }
    }
}
