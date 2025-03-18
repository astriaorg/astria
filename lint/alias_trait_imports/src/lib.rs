#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_hir::{
    def::{
        DefKind,
        Res,
    },
    Arm,
    Block,
    Body,
    Closure,
    Expr,
    ExprKind,
    FnDecl,
    FnRetTy,
    FnSig,
    GenericArg,
    GenericArgs,
    GenericBound,
    GenericParam,
    GenericParamKind,
    Generics,
    ImplItem,
    ImplItemKind,
    Item,
    ItemKind,
    LetStmt,
    Mod,
    Node,
    Pat,
    PatKind,
    Path,
    PathSegment,
    PolyTraitRef,
    QPath,
    Stmt,
    StmtKind,
    TraitFn,
    TraitItem,
    TraitItemKind,
    TraitRef,
    Ty,
    TyKind,
    UseKind,
    VariantData,
    WherePredicate,
};
use rustc_lint::{
    LateContext,
    LateLintPass,
    LintContext as _,
};
use rustc_middle::{
    bug,
    hir::map::Map,
    ty::Visibility,
};
use rustc_span::{
    symbol::{
        kw,
        Symbol,
    },
    FileName,
    FileNameDisplayPreference,
};

dylint_linting::declare_late_lint! {
    /// ### What it does
    /// Checks if a trait is imported without an alias, but is not explicitly named in the code.
    ///
    /// ### Why is this bad?
    /// Importing a trait without aliasing can lead to namespace pollution.
    ///
    /// ### Example
    /// ```rust
    /// // `Write` trait is imported but not aliased
    /// use std::fmt::Write;
    ///
    /// let mut out_string = String::new();
    /// writeln!(&mut out_string, "Hello, world!");
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::fmt::Write as _;
    ///
    /// let mut out_string = String::new();
    /// writeln!(&mut out_string, "Hello, world!");
    /// ```
    pub ALIAS_TRAIT_IMPORTS,
    Warn,
    "Checks if traits which are imported but not explicitly named in the code are aliased `as _`."
}

impl<'tcx> LateLintPass<'tcx> for AliasTraitImports {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Ignore generated files
        let file_name = cx.sess().source_map().span_to_filename(item.span);
        if let FileName::Real(path) = file_name {
            if path
                .to_string_lossy(FileNameDisplayPreference::Local)
                .contains("crates/astria-core/src/generated")
            {
                return;
            }
        }

        let hir_map = cx.tcx.hir();
        if let ItemKind::Use(path, UseKind::Single) = item.kind {
            // Check if the import is a trait
            if let Res::Def(DefKind::Trait, _) = path.res[0] {
                // Check if the trait is a re-export, ignoring if it is
                let parent_def_id = hir_map.get_parent_item(item.hir_id()).to_def_id();
                let parent = hir_map
                    .get_if_local(parent_def_id.into())
                    .expect("parent of `Use` item should be a local item");
                match parent {
                    Node::Item(Item {
                        kind: ItemKind::Fn(..),
                        ..
                    }) => {} /* If the parent is a function, the trait cannot be a re-export and
                              * this check is skipped */
                    _ => {
                        // Check if this is a re-export, ignore if it is
                        let visibility = cx.tcx.visibility(item.owner_id.to_def_id());
                        match visibility {
                            Visibility::Restricted(restricted_id) => {
                                if cx.tcx.def_path_hash(restricted_id)
                                    != cx.tcx.def_path_hash(parent_def_id)
                                {
                                    return; // If the visibility is higher, this is a re-export
                                }
                            }
                            Visibility::Public => {
                                return;
                            }
                        }
                    }
                }

                // `span_snippet` is used to determine trait name, check for aliasing, and provide
                // suggestion if lint is triggered
                let Ok(mut span_snippet) = cx.sess().source_map().span_to_snippet(item.span) else {
                    bug!("failed to extract source text from `Use` item")
                };

                // If trait is already aliased `as _`, ignore
                if span_snippet.contains(" as _") {
                    return;
                }

                // Extract trait name
                let mut trait_name = match path.segments.last() {
                    Some(segment) if segment.ident.name != kw::Underscore => segment.ident.name,
                    None | Some(_) => return,
                };

                // If the trait is aliased as something other than `_`, assign `trait_name` to alias
                // and trim the alias from the span snippet for correct lint
                // suggestion
                if trait_name != item.ident.name && span_snippet.contains(" as ") {
                    let mut semicolon_suffix = false;
                    let mut suffix_to_strip = format!(" as {}", item.ident.name);
                    if span_snippet.ends_with(';') {
                        semicolon_suffix = true;
                        suffix_to_strip.push(';');
                    }
                    span_snippet = span_snippet
                        .strip_suffix(&suffix_to_strip)
                        .expect(&format!(
                            "aliased trait import should have the suffix \"{trait_name} as {}\"",
                            item.ident.name
                        ))
                        .to_string();
                    trait_name = item.ident.name;
                    if semicolon_suffix {
                        span_snippet.push(';');
                    }
                }

                // These are the only possible parents for a `Use` item
                let found = match parent {
                    Node::Crate(module) => find_in_mod(module, trait_name, hir_map),
                    Node::Item(item) => find_in_item(item, trait_name, hir_map),
                    Node::Block(block) => find_in_block(block, trait_name, hir_map),
                    Node::Stmt(stmt) => find_in_stmt(stmt, trait_name, hir_map),
                    Node::TraitItem(trait_item) => {
                        find_in_trait_item(trait_item, trait_name, hir_map)
                    }
                    Node::ImplItem(impl_item) => find_in_impl_item(impl_item, trait_name, hir_map),
                    _ => panic!("unexpected parent of `Use` item"),
                };

                // Check if the trait name is explicitly used
                if !found {
                    // Semicolons are included in the span snippet, so they must be handled
                    // accordingly, whereas commas are not
                    let sugg = if span_snippet.ends_with(';') {
                        format!("{} as _;", span_snippet.strip_suffix(';').unwrap())
                    } else {
                        format!("{span_snippet} as _")
                    };
                    span_lint_and_sugg(
                        cx,
                        ALIAS_TRAIT_IMPORTS,
                        item.span,
                        format!(
                            "imported but unmentioned trait `{trait_name}` should be imported `as \
                             _`",
                        ),
                        "consider importing the trait `as _` to avoid namespace pollution",
                        sugg,
                        rustc_errors::Applicability::MachineApplicable,
                    );
                }
            }
        }
    }
}

fn find_in_mod(module: &'_ Mod<'_>, trait_name: Symbol, hir: Map) -> bool {
    module
        .item_ids
        .iter()
        .any(|item_id| find_in_item(hir.item(*item_id), trait_name, hir))
}

fn find_in_item(item: &'_ Item<'_>, trait_name: Symbol, hir: Map) -> bool {
    match item.kind {
        ItemKind::Mod(module) => find_in_mod(module, trait_name, hir),
        ItemKind::Fn(sig, generics, body_id) => {
            find_in_fn_sig(sig, trait_name, hir)
                || find_in_generics(generics, trait_name, hir)
                || find_in_body(hir.body(body_id), trait_name, hir)
        }
        ItemKind::Static(ty, _, body_id) => {
            find_in_ty(ty, trait_name, hir) || find_in_body(hir.body(body_id), trait_name, hir)
        }
        ItemKind::Const(ty, generics, body_id) => {
            find_in_ty(ty, trait_name, hir)
                || find_in_generics(generics, trait_name, hir)
                || find_in_body(hir.body(body_id), trait_name, hir)
        }
        ItemKind::TyAlias(ty, generics) => {
            find_in_ty(ty, trait_name, hir) || find_in_generics(generics, trait_name, hir)
        }
        ItemKind::Enum(_, generics) | ItemKind::Union(_, generics) => {
            find_in_generics(generics, trait_name, hir)
        }
        ItemKind::Struct(variant_data, generics) => {
            find_in_variant_data(variant_data, trait_name, hir)
                || find_in_generics(generics, trait_name, hir)
        }
        ItemKind::Trait(_, _, generics, generic_bounds, trait_item_refs) => {
            find_in_generics(generics, trait_name, hir)
                || find_in_generic_bounds(generic_bounds, trait_name, hir)
                || trait_item_refs.iter().any(|trait_item_ref| {
                    find_in_trait_item(hir.trait_item(trait_item_ref.id), trait_name, hir)
                })
        }
        ItemKind::TraitAlias(generics, generic_bounds) => {
            find_in_generics(generics, trait_name, hir)
                || find_in_generic_bounds(generic_bounds, trait_name, hir)
        }
        ItemKind::Impl(impl_statement) => {
            impl_statement
                .items
                .iter()
                .any(|item| find_in_impl_item(hir.impl_item(item.id), trait_name, hir))
                || find_in_generics(impl_statement.generics, trait_name, hir)
                || impl_statement.of_trait.as_ref().map_or(false, |trait_ref| {
                    find_in_trait_ref(trait_ref, trait_name, hir)
                })
        }
        ItemKind::OpaqueTy(opaque_ty) => {
            find_in_generics(opaque_ty.generics, trait_name, hir)
                || find_in_generic_bounds(opaque_ty.bounds, trait_name, hir)
        }
        ItemKind::ExternCrate(_)
        | ItemKind::Use(..)
        | ItemKind::Macro(..)
        | ItemKind::ForeignMod {
            abi: _,
            items: _,
        }
        | ItemKind::GlobalAsm(_) => false,
    }
}

fn find_in_variant_data(variant_data: VariantData, trait_name: Symbol, hir: Map) -> bool {
    match variant_data {
        VariantData::Struct {
            fields, ..
        } => fields
            .iter()
            .any(|field| find_in_ty(field.ty, trait_name, hir)),
        VariantData::Tuple(fields, ..) => fields
            .iter()
            .any(|field| find_in_ty(field.ty, trait_name, hir)),
        VariantData::Unit(..) => false,
    }
}

fn find_in_body(body: &'_ Body<'_>, trait_name: Symbol, hir: Map) -> bool {
    body.params
        .iter()
        .any(|param| find_in_pat(param.pat, trait_name, hir))
        || find_in_expr(body.value, trait_name, hir)
}

fn find_in_trait_item(trait_item: &'_ TraitItem<'_>, trait_name: Symbol, hir: Map) -> bool {
    find_in_generics(trait_item.generics, trait_name, hir)
        || match trait_item.kind {
            TraitItemKind::Const(ty, _) => find_in_ty(ty, trait_name, hir),
            TraitItemKind::Fn(fn_sig, trait_fn) => {
                find_in_fn_sig(fn_sig, trait_name, hir)
                    || find_in_trait_fn(trait_fn, trait_name, hir)
            }
            TraitItemKind::Type(bounds, ty) => {
                find_in_generic_bounds(bounds, trait_name, hir)
                    || ty.map_or(false, |ty| find_in_ty(ty, trait_name, hir))
            }
        }
}

fn find_in_trait_fn(trait_fn: TraitFn, trait_name: Symbol, hir: Map) -> bool {
    match trait_fn {
        TraitFn::Required(idents) => idents.iter().any(|ident| ident.name == trait_name),
        TraitFn::Provided(body_id) => find_in_body(hir.body(body_id), trait_name, hir),
    }
}

fn find_in_impl_item(impl_item: &'_ ImplItem<'_>, trait_name: Symbol, hir: Map) -> bool {
    find_in_generics(impl_item.generics, trait_name, hir)
        || match impl_item.kind {
            ImplItemKind::Const(ty, _) | ImplItemKind::Type(ty) => find_in_ty(ty, trait_name, hir),
            ImplItemKind::Fn(fn_sig, body_id) => {
                find_in_fn_sig(fn_sig, trait_name, hir)
                    || find_in_body(hir.body(body_id), trait_name, hir)
            }
        }
}

fn find_in_fn_sig(fn_sig: FnSig, trait_name: Symbol, hir: Map) -> bool {
    find_in_fn_decl(fn_sig.decl, trait_name, hir)
}

fn find_in_fn_decl(fn_decl: &'_ FnDecl<'_>, trait_name: Symbol, hir: Map) -> bool {
    fn_decl
        .inputs
        .iter()
        .any(|input| find_in_ty(input, trait_name, hir))
        || match fn_decl.output {
            FnRetTy::Return(ty) => find_in_ty(ty, trait_name, hir),
            FnRetTy::DefaultReturn(_) => false,
        }
}

fn find_in_block(block: &'_ Block<'_>, trait_name: Symbol, hir: Map) -> bool {
    block
        .stmts
        .iter()
        .any(|stmt| find_in_stmt(stmt, trait_name, hir))
        || block
            .expr
            .map_or(false, |expr| find_in_expr(expr, trait_name, hir))
}

fn find_in_stmt(stmt: &'_ Stmt<'_>, trait_name: Symbol, hir: Map) -> bool {
    match stmt.kind {
        StmtKind::Let(let_stmt) => find_in_let_stmt(let_stmt, trait_name, hir),
        StmtKind::Item(item) => find_in_item(hir.item(item), trait_name, hir),
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => find_in_expr(expr, trait_name, hir),
    }
}

fn find_in_expr(expr: &'_ Expr<'_>, trait_name: Symbol, hir: Map) -> bool {
    match expr.kind {
        ExprKind::Path(qpath) => find_in_qpath(qpath, trait_name, hir),
        ExprKind::Call(expr, exprs) => {
            find_in_expr(expr, trait_name, hir)
                || exprs.iter().any(|expr| find_in_expr(expr, trait_name, hir))
        }
        ExprKind::MethodCall(path_segment, expr, exprs, _) => {
            find_in_path_segment(path_segment, trait_name, hir)
                || find_in_expr(expr, trait_name, hir)
                || exprs.iter().any(|expr| find_in_expr(expr, trait_name, hir))
        }
        ExprKind::Struct(qpath, fields, _) => {
            find_in_qpath(*qpath, trait_name, hir)
                || fields
                    .iter()
                    .any(|field| find_in_expr(field.expr, trait_name, hir))
        }
        ExprKind::Field(expr, _) => find_in_expr(expr, trait_name, hir),
        ExprKind::Cast(expr, ty) | ExprKind::Type(expr, ty) => {
            find_in_expr(expr, trait_name, hir) || find_in_ty(ty, trait_name, hir)
        }
        ExprKind::Block(block, _) => find_in_block(block, trait_name, hir),
        ExprKind::Closure(closure) => find_in_closure(closure, trait_name, hir),
        ExprKind::DropTemps(expr) => find_in_expr(expr, trait_name, hir),
        ExprKind::AddrOf(_, _, expr) => find_in_expr(expr, trait_name, hir),
        ExprKind::Tup(exprs) | ExprKind::Array(exprs) => {
            exprs.iter().any(|expr| find_in_expr(expr, trait_name, hir))
        }
        ExprKind::If(cond, then, els) => {
            find_in_expr(cond, trait_name, hir)
                || find_in_expr(then, trait_name, hir)
                || els.map_or(false, |els| find_in_expr(els, trait_name, hir))
        }
        ExprKind::Match(expr, arms, _) => {
            find_in_expr(expr, trait_name, hir)
                || arms.iter().any(|arm| find_in_arm(arm, trait_name, hir))
        }
        _ => false,
    }
}

fn find_in_arm(arm: &'_ Arm<'_>, trait_name: Symbol, hir: Map) -> bool {
    find_in_pat(arm.pat, trait_name, hir)
        || arm
            .guard
            .map_or(false, |guard| find_in_expr(guard, trait_name, hir))
        || find_in_expr(arm.body, trait_name, hir)
}

fn find_in_closure(closure: &'_ Closure<'_>, trait_name: Symbol, hir: Map) -> bool {
    find_in_fn_decl(closure.fn_decl, trait_name, hir)
        || find_in_generic_params(closure.bound_generic_params, trait_name, hir)
        || find_in_body(hir.body(closure.body), trait_name, hir)
}

fn find_in_let_stmt(let_stmt: &'_ LetStmt<'_>, trait_name: Symbol, hir: Map) -> bool {
    let_stmt
        .ty
        .map_or(false, |ty| find_in_ty(ty, trait_name, hir))
        || let_stmt
            .init
            .map_or(false, |expr| find_in_expr(expr, trait_name, hir))
        || let_stmt
            .els
            .map_or(false, |els| find_in_block(els, trait_name, hir))
        || find_in_pat(let_stmt.pat, trait_name, hir)
}

fn find_in_pat(pat: &'_ Pat<'_>, trait_name: Symbol, hir: Map) -> bool {
    match pat.kind {
        PatKind::Path(qpath) => find_in_qpath(qpath, trait_name, hir),
        PatKind::Struct(qpath, fields, _) => {
            find_in_qpath(qpath, trait_name, hir)
                || fields
                    .iter()
                    .any(|field| find_in_pat(field.pat, trait_name, hir))
        }
        PatKind::TupleStruct(qpath, pats, _) => {
            find_in_qpath(qpath, trait_name, hir)
                || pats.iter().any(|pat| find_in_pat(pat, trait_name, hir))
        }
        PatKind::Or(pats) => pats.iter().any(|pat| find_in_pat(pat, trait_name, hir)),
        PatKind::Tuple(pats, _) => pats.iter().any(|pat| find_in_pat(pat, trait_name, hir)),
        PatKind::Box(pat) => find_in_pat(pat, trait_name, hir),
        PatKind::Wild
        | PatKind::Lit(_)
        | PatKind::Range(..)
        | PatKind::Binding(..)
        | PatKind::Never
        | PatKind::Ref(..)
        | PatKind::Deref(_)
        | PatKind::Slice(..)
        | PatKind::Err(_) => false,
    }
}

fn find_in_qpath(qpath: QPath, trait_name: Symbol, hir: Map) -> bool {
    match qpath {
        QPath::Resolved(ty, path) => {
            find_in_path(path, trait_name, hir)
                || ty.map_or(false, |ty| find_in_ty(ty, trait_name, hir))
        }
        QPath::TypeRelative(ty, path_segment) => {
            find_in_ty(ty, trait_name, hir) || find_in_path_segment(path_segment, trait_name, hir)
        }
        QPath::LangItem(..) => false,
    }
}

fn find_in_generics(generics: &'_ Generics<'_>, trait_name: Symbol, hir: Map) -> bool {
    find_in_generic_params(generics.params, trait_name, hir)
        || generics
            .predicates
            .iter()
            .any(|predicate| find_in_where_predicate(predicate, trait_name, hir))
}

fn find_in_where_predicate(
    where_predicate: &'_ WherePredicate<'_>,
    trait_name: Symbol,
    hir: Map,
) -> bool {
    match where_predicate {
        WherePredicate::BoundPredicate(predicate) => {
            find_in_generic_params(predicate.bound_generic_params, trait_name, hir)
                || find_in_ty(predicate.bounded_ty, trait_name, hir)
                || find_in_generic_bounds(predicate.bounds, trait_name, hir)
        }
        WherePredicate::RegionPredicate(_) | WherePredicate::EqPredicate(_) => false,
    }
}

fn find_in_ty(ty: &'_ Ty<'_>, trait_name: Symbol, hir: Map) -> bool {
    match ty.kind {
        TyKind::TraitObject(trait_refs, ..) => trait_refs
            .iter()
            .any(|(poly_trait_ref, _)| find_in_poly_trait_ref(poly_trait_ref, trait_name, hir)),
        TyKind::OpaqueDef(item_id, ..) => hir.item(item_id).ident.name == trait_name,
        TyKind::Path(qpath) => match qpath {
            QPath::Resolved(ty, path) => {
                find_in_path(path, trait_name, hir)
                    || ty.map_or(false, |ty| find_in_ty(ty, trait_name, hir))
            }
            QPath::TypeRelative(ty, path_segment) => {
                find_in_ty(ty, trait_name, hir)
                    || find_in_path_segment(path_segment, trait_name, hir)
            }
            QPath::LangItem(..) => false,
        },
        TyKind::Slice(ty) | TyKind::Array(ty, _) => find_in_ty(ty, trait_name, hir),
        TyKind::Ptr(mut_ty) | TyKind::Ref(_, mut_ty) => find_in_ty(mut_ty.ty, trait_name, hir),
        TyKind::Tup(tys) => tys.iter().any(|ty| find_in_ty(ty, trait_name, hir)),
        TyKind::InferDelegation(..)
        | TyKind::Never
        | TyKind::AnonAdt(_)
        | TyKind::Typeof(_)
        | TyKind::Infer
        | TyKind::Err(_)
        | TyKind::BareFn(_)
        | TyKind::Pat(..) => false,
    }
}

fn find_in_generic_bounds(
    generic_bounds: &'_ [GenericBound<'_>],
    trait_name: Symbol,
    hir: Map,
) -> bool {
    generic_bounds.iter().any(|bound| match bound {
        GenericBound::Trait(poly_trait_ref, _modifier) => {
            find_in_poly_trait_ref(poly_trait_ref, trait_name, hir)
        }
        GenericBound::Outlives(_) | GenericBound::Use(..) => false,
    })
}

fn find_in_generic_params(
    generic_params: &'_ [GenericParam<'_>],
    trait_name: Symbol,
    hir: Map,
) -> bool {
    generic_params
        .iter()
        .any(|generic_param| find_in_generic_param(generic_param, trait_name, hir))
}

fn find_in_generic_param(
    generic_param: &'_ GenericParam<'_>,
    trait_name: Symbol,
    hir: Map,
) -> bool {
    match generic_param.kind {
        GenericParamKind::Type {
            default,
            synthetic: _,
        } => default.map_or(false, |ty| find_in_ty(ty, trait_name, hir)),
        GenericParamKind::Const {
            ty,
            default: _,
            is_host_effect: _,
            synthetic: _,
        } => find_in_ty(ty, trait_name, hir),
        GenericParamKind::Lifetime {
            kind: _,
        } => false,
    }
}

fn find_in_poly_trait_ref(
    poly_trait_ref: &'_ PolyTraitRef<'_>,
    trait_name: Symbol,
    hir: Map,
) -> bool {
    find_in_generic_params(poly_trait_ref.bound_generic_params, trait_name, hir)
        || find_in_trait_ref(&poly_trait_ref.trait_ref, trait_name, hir)
}

fn find_in_trait_ref(trait_ref: &'_ TraitRef<'_>, trait_name: Symbol, hir: Map) -> bool {
    find_in_path(trait_ref.path, trait_name, hir)
}

fn find_in_path(path: &'_ Path<'_>, trait_name: Symbol, hir: Map) -> bool {
    path.segments
        .iter()
        .any(|segment| find_in_path_segment(segment, trait_name, hir))
}

fn find_in_generic_args(generic_args: &'_ GenericArgs<'_>, trait_name: Symbol, hir: Map) -> bool {
    generic_args.args.iter().any(|arg| match arg {
        GenericArg::Type(ty) => find_in_ty(ty, trait_name, hir),
        GenericArg::Lifetime(_) | GenericArg::Const(_) | GenericArg::Infer(_) => false,
    })
}

fn find_in_path_segment(path_segment: &'_ PathSegment<'_>, trait_name: Symbol, hir: Map) -> bool {
    path_segment
        .args
        .map_or(false, |args| find_in_generic_args(args, trait_name, hir))
        || path_segment.ident.name == trait_name
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
