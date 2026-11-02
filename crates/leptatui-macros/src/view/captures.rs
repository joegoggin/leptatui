//! Owned captures for compiler-generated reactive expressions.
//!
//! Signal tracking happens at runtime. This visitor only clones free value
//! references before moving them into generated closures, respecting local
//! bindings and leaving function and type paths untouched.

use std::collections::{BTreeMap, HashSet};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, Pat, Stmt, Token,
    punctuated::Punctuated,
    visit::{self, Visit},
};

/// Free value references and lexical bindings in a deferred expression.
#[derive(Default)]
struct Captures {
    /// Unique names to clone outside the generated closure.
    values: BTreeMap<String, Ident>,
    /// Names bound by each surrounding Rust lexical scope.
    scopes: Vec<HashSet<String>>,
}

impl Captures {
    /// Records all names introduced by a Rust pattern in the current scope.
    ///
    /// # Arguments
    ///
    /// * `pattern` — Destructuring or identifier pattern introducing bindings.
    fn bind(&mut self, pattern: &Pat) {
        /// Collects identifier bindings from a destructuring pattern.
        struct Names(Vec<String>);
        impl<'ast> Visit<'ast> for Names {
            fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
                self.0.push(pattern.ident.to_string());
                visit::visit_pat_ident(self, pattern);
            }
        }
        let mut names = Names(Vec::new());
        names.visit_pat(pattern);
        self.scopes
            .last_mut()
            .expect("capture scope missing")
            .extend(names.0);
    }
}

impl<'ast> Visit<'ast> for Captures {
    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        if expr.qself.is_none() && expr.path.segments.len() == 1 {
            let ident = &expr.path.segments[0].ident;
            let name = ident.to_string();
            if name != "self" && !self.scopes.iter().rev().any(|scope| scope.contains(&name)) {
                self.values.entry(name).or_insert_with(|| ident.clone());
            }
        }
    }

    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        // A bare function name may be generic and cannot be cloned before its
        // call establishes its type arguments. Captured callable receivers can
        // still use explicit `.clone()` or method invocation.
        if !matches!(expr.func.as_ref(), Expr::Path(_)) {
            self.visit_expr(&expr.func);
        }
        for arg in &expr.args {
            self.visit_expr(arg);
        }
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scopes.push(HashSet::new());
        for statement in &block.stmts {
            if let Stmt::Local(local) = statement {
                if let Some(init) = &local.init {
                    self.visit_expr(&init.expr);
                    if let Some((_, diverge)) = &init.diverge {
                        self.visit_expr(diverge);
                    }
                }
                self.bind(&local.pat);
            } else {
                self.visit_stmt(statement);
            }
        }
        self.scopes.pop();
    }

    fn visit_expr_closure(&mut self, expr: &'ast syn::ExprClosure) {
        self.scopes.push(HashSet::new());
        for input in &expr.inputs {
            self.bind(input);
        }
        self.visit_expr(&expr.body);
        self.scopes.pop();
    }

    fn visit_expr_for_loop(&mut self, expr: &'ast syn::ExprForLoop) {
        self.visit_expr(&expr.expr);
        self.scopes.push(HashSet::new());
        self.bind(&expr.pat);
        self.visit_block(&expr.body);
        self.scopes.pop();
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.scopes.push(HashSet::new());
        self.bind(&arm.pat);
        if let Some((_, guard)) = &arm.guard {
            self.visit_expr(guard);
        }
        self.visit_expr(&arm.body);
        self.scopes.pop();
    }

    fn visit_expr_if(&mut self, expr: &'ast syn::ExprIf) {
        self.scopes.push(HashSet::new());
        self.visit_expr(&expr.cond);
        self.visit_block(&expr.then_branch);
        self.scopes.pop();
        if let Some((_, branch)) = &expr.else_branch {
            self.visit_expr(branch);
        }
    }

    fn visit_expr_let(&mut self, expr: &'ast syn::ExprLet) {
        self.visit_expr(&expr.expr);
        self.bind(&expr.pat);
    }

    fn visit_expr_while(&mut self, expr: &'ast syn::ExprWhile) {
        self.scopes.push(HashSet::new());
        self.visit_expr(&expr.cond);
        self.visit_block(&expr.body);
        self.scopes.pop();
    }

    fn visit_expr_macro(&mut self, expr: &'ast syn::ExprMacro) {
        let Some(name) = expr
            .mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if name == "vec" {
            if let Ok(array) = {
                let tokens = &expr.mac.tokens;
                syn::parse2::<Expr>(quote! { [#tokens] })
            } {
                self.visit_expr(&array);
            }
            // A comma-separated vector shares expression-list syntax with format arguments.
        }
        if !matches!(name.as_str(), "format" | "format_args" | "vec") {
            return;
        }
        use syn::parse::Parser;
        let Ok(args) =
            Punctuated::<Expr, Token![,]>::parse_terminated.parse2(expr.mac.tokens.clone())
        else {
            return;
        };
        let explicit = args
            .iter()
            .filter_map(|arg| {
                if let Expr::Assign(assign) = arg {
                    if let Expr::Path(path) = assign.left.as_ref() {
                        return path.path.get_ident().map(ToString::to_string);
                    }
                }
                None
            })
            .collect::<HashSet<_>>();
        for arg in &args {
            if let Expr::Assign(assign) = arg {
                self.visit_expr(&assign.right);
            } else {
                self.visit_expr(arg);
            }
        }
        if name == "vec" {
            return;
        }
        let Some(Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(format),
            ..
        })) = args.first()
        else {
            return;
        };
        let format_value = format.value();
        let mut chars = format_value.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '{' {
                continue;
            }
            if chars.peek() == Some(&'{') {
                chars.next();
                continue;
            }
            let mut name = String::new();
            while chars
                .peek()
                .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_')
            {
                name.push(chars.next().unwrap());
            }
            if explicit.contains(&name) || self.scopes.iter().any(|scope| scope.contains(&name)) {
                continue;
            }
            if let Ok(mut ident) = syn::parse_str::<Ident>(&name) {
                ident.set_span(format.span());
                self.values.entry(name).or_insert(ident);
            }
        }
    }
}

/// Clones free value references before creating a generated tracking closure.
///
/// # Arguments
///
/// * `expressions` — Deferred expressions sharing the generated closure.
///
/// # Returns
///
/// A [`TokenStream`] of local capture declarations.
pub(super) fn capture_values<'a>(expressions: impl IntoIterator<Item = &'a Expr>) -> TokenStream {
    let mut visitor = Captures::default();
    visitor.scopes.push(HashSet::new());
    for expression in expressions {
        visitor.visit_expr(expression);
    }
    let names = visitor.values.values();
    quote! { #(let #names = ::core::clone::Clone::clone(&#names);)* }
}
