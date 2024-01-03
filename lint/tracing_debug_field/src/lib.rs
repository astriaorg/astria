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
            then {
                let span = first_span_in_crate(arg);
                span_lint_and_help(
                    cx,
                    TRACING_DEBUG_FIELD,
                    span,
                    "tracing events must not contain debug-formatted fields",
                    None,
                    "emit the std::fmt::Display format of the object using the % sigil. \
                    You might have to implement the Display trait or serialize the object \
                    to a format that can be written as a string (like JSON). Consider if \
                    emitting the entire object is necessary or if the information can be \
                    reduced."
                )
            }
        );
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

fn first_span_in_crate(arg: &Expr<'_>) -> Span {
    let mut span = 'get_span: {
        // Case 1: fields like foo = ?bar that are transformed as debug(&bar).
        if let Expr {
            kind:
                ExprKind::AddrOf(
                    _,
                    _,
                    Expr {
                        span, ..
                    },
                ),
            ..
        } = arg
        {
            break 'get_span *span;
        };
        // Case 2: fields like foo = tracing::field::debug(bar) or the shorthand ?bar.
        // These either point to the actual source as they are evaluated eagerly (first case),
        // or are expanded inside the tracing::event! macro (short case).
        arg.span
    };

    // Find the first span that is not from an expansion.
    // While the cases `foo = ?bar` and `foo = debug(bar)` yield spans that directly point
    // to source code, the shorthand `?bar` does not. Instead one has to loop over its expansion
    // data until the first call site that is inside the crate source is found. This is usually
    // the entire tracing::event! macro.
    while span.from_expansion() {
        span = span.ctxt().outer_expn_data().call_site;
    }
    span
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}
