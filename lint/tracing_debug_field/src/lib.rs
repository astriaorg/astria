#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::{
    diagnostics::span_lint_and_help,
    is_expr_path_def_path,
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
use rustc_span::{
    hygiene::{
        ExpnKind,
        MacroKind,
    },
    Span,
};

const TRACING_FIELD_DEBUG: [&str; 3] = ["tracing_core", "field", "debug"];

dylint_linting::impl_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRACING_DEBUG_FIELD,
    Warn,
    "use of debug formatted field in tracing event macro",
    TracingDebugField::default()
}

#[derive(Default)]
struct TracingDebugField {
    event_span_depth: usize,
}

impl<'tcx> LateLintPass<'tcx> for TracingDebugField {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if is_in_tracing_event_macro(expr.span) {
            self.event_span_depth += 1;
        }
        if_chain!(
            // match function calls, i.e. expressions f(...args)
            if let ExprKind::Call(callee, args) = expr.kind;
            // match tracing_core::field::debug
            if is_expr_path_def_path(cx, callee, &TRACING_FIELD_DEBUG);
            // match only tracing::event! declarative macros ignoring instrument attributes;
            // if `event_span_depth > 0` this means we are in a tracing::event! macro
            if self.event_span_depth > 0;
            // tracing_core::field::debug has one argument
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

    fn check_expr_post(&mut self, _cx: &LateContext<'tcx>, expr: &Expr<'_>) {
        if is_in_tracing_event_macro(expr.span) {
            self.event_span_depth -= 1;
        }
    }
}

fn is_in_tracing_event_macro(span: Span) -> bool {
    if_chain!(
        if let ExpnKind::Macro(MacroKind::Bang, name) = span.ctxt().outer_expn_data().kind;
        if name.as_str() == "$crate::event";
        then {
            return true;
        }
    );
    false
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
