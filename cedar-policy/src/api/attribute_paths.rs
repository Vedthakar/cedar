/*
 * Copyright Cedar Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Attribute access paths used by a policy (#2127)

use std::collections::BTreeSet;
use std::fmt;

use cedar_policy_core::ast::{Expr, ExprKind, Literal, Var};
use smol_str::SmolStr;

use crate::EntityUid;

/// Where an [`AttributePath`] starts.
#[doc = include_str!("../../experimental_warning.md")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AttributePathRoot {
    /// The `principal` variable
    Principal,
    /// The `action` variable
    Action,
    /// The `resource` variable
    Resource,
    /// The `context` variable
    Context,
    /// An entity literal, e.g. `User::"alice"`
    Entity(EntityUid),
}

/// A chain of attribute accesses from a request variable or an entity literal
/// that a policy uses, e.g. `resource.owner.name` in
/// `resource.owner.name == "alice"` or `resource has owner.name`.
#[doc = include_str!("../../experimental_warning.md")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributePath {
    root: AttributePathRoot,
    attrs: Vec<SmolStr>,
}

impl AttributePath {
    /// Where the path starts
    pub fn root(&self) -> &AttributePathRoot {
        &self.root
    }

    /// The attribute names along the path, outermost first; never empty
    pub fn attrs(&self) -> impl Iterator<Item = &str> {
        self.attrs.iter().map(SmolStr::as_str)
    }
}

/// Writes the path in Cedar syntax, e.g. `principal.name` or
/// `User::"alice"["first name"]`.
impl fmt::Display for AttributePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let root = match &self.root {
            AttributePathRoot::Principal => Expr::var(Var::Principal),
            AttributePathRoot::Action => Expr::var(Var::Action),
            AttributePathRoot::Resource => Expr::var(Var::Resource),
            AttributePathRoot::Context => Expr::var(Var::Context),
            AttributePathRoot::Entity(euid) => Expr::val(euid.as_ref().clone()),
        };
        let expr = self
            .attrs
            .iter()
            .fold(root, |expr, attr| Expr::get_attr(expr, attr.clone()));
        write!(f, "{expr}")
    }
}

/// The root and attributes of `expr` if it is a chain of attribute accesses
/// from a variable or an entity literal.
fn path_of(expr: &Expr) -> Option<(AttributePathRoot, Vec<SmolStr>)> {
    match expr.expr_kind() {
        ExprKind::Var(var) => Some((
            match var {
                Var::Principal => AttributePathRoot::Principal,
                Var::Action => AttributePathRoot::Action,
                Var::Resource => AttributePathRoot::Resource,
                Var::Context => AttributePathRoot::Context,
            },
            Vec::new(),
        )),
        ExprKind::Lit(Literal::EntityUID(euid)) => Some((
            AttributePathRoot::Entity(EntityUid::from(euid.as_ref().clone())),
            Vec::new(),
        )),
        ExprKind::GetAttr { expr, attr } => path_of(expr).map(|(root, mut attrs)| {
            attrs.push(attr.clone());
            (root, attrs)
        }),
        _ => None,
    }
}

/// Every attribute path that `condition` reads or tests with `has`
pub(crate) fn attribute_paths(condition: &Expr) -> BTreeSet<AttributePath> {
    let mut paths = BTreeSet::new();
    for expr in condition.subexpressions() {
        let (base, attrs): (&Expr, Vec<&SmolStr>) = match expr.expr_kind() {
            ExprKind::GetAttr { expr, attr } | ExprKind::HasAttr { expr, attr } => {
                (expr, vec![attr])
            }
            ExprKind::ExtHasAttr { expr, attrs } => (expr, attrs.iter().collect()),
            _ => continue,
        };
        if let Some((root, mut path)) = path_of(base) {
            for attr in attrs {
                path.push(attr.clone());
                paths.insert(AttributePath {
                    root: root.clone(),
                    attrs: path.clone(),
                });
            }
        }
    }
    paths
}
