#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;

use clippy_utils::{
    diagnostics::span_lint_and_help,
    is_expr_path_def_path,
    is_from_proc_macro,
};
use if_chain::if_chain;
use rustc_hir::{
    Expr,
    ExprKind,
};
use rustc_lint::{
    LateContext,
    LateLintPass,
};

const TRACING_FIELD_DEBUG: [&str; 3] = ["tracing_core", "field", "debug"];

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Known problems
    /// Remove if none.
    ///
    /// ### Example
    /// ```rust
    /// // example code where a warning is issued
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code that does not raise a warning
    /// ```
    pub TRACING_DEBUG_FIELD,
    Warn,
    "description goes here"
}

impl<'tcx> LateLintPass<'tcx> for TracingDebugField {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if_chain!(
            if let ExprKind::Call(callee, args) = expr.kind;
            if is_expr_path_def_path(cx, callee, &TRACING_FIELD_DEBUG);
            if !is_from_proc_macro(cx, expr);
            if let Some(arg) = args.first();

            if let
            // case 1: using the ? sigil. We know that the tracing macros
            // will transform `?foo` into `debug(&foo)`.
             Expr { kind: ExprKind::AddrOf(_, _, Expr { span, ..}), .. }
            // case2 : using tracing_core::field::debug directly. This is
            // evaluated eagerly and the span points to the actual source,
            // not to the generated code.
            | Expr { span, ..}
            = arg;
            then {
                span_lint_and_help(
                    cx,
                    TRACING_DEBUG_FIELD,
                    *span,
                    "tracing events must not contain debug-formatted fields",
                    None,
                    "implement the std::fmt::Display trait or a newtype wrapper, and use tracing::field::display or the `%` sigil instead",
                )
            }
        )
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
