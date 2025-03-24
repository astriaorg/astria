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
                // If the parent is a function, the trait cannot be a re-export and the visibility
                // check is skipped
                if matches!(
                    parent,
                    Node::Item(Item {
                        kind: ItemKind::Fn(..),
                        ..
                    })
                ) {
                    return;
                } else {
                    // Check if this is a re-export, ignore if it is
                    let visibility = cx.tcx.visibility(item.owner_id.to_def_id());
                    let Visibility::Restricted(restricted_id) = visibility else {
                        return;
                    };
                    if cx.tcx.def_path_hash(restricted_id) != cx.tcx.def_path_hash(parent_def_id) {
                        return; // If the visibility is higher, this is a re-export
                    }
                }

                // `span_snippet` is used to determine trait name, check for aliasing, and provide
                // suggestion if lint is triggered
                let Ok(mut span_snippet) = cx.sess().source_map().span_to_snippet(item.span) else {
                    panic!("failed to extract source text from `Use` item")
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
                    Node::Crate(module) => module.find(trait_name, &hir_map),
                    Node::Item(item) => item.find(trait_name, &hir_map),
                    Node::Block(block) => block.find(trait_name, &hir_map),
                    Node::Stmt(stmt) => stmt.find(trait_name, &hir_map),
                    Node::TraitItem(trait_item) => trait_item.find(trait_name, &hir_map),
                    Node::ImplItem(impl_item) => impl_item.find(trait_name, &hir_map),
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
                            "Trait `{trait_name}` is imported but not explicitly used. Consider \
                             `{trait_name} as _`.",
                        ),
                        format!(
                            "Consider importing the trait `as _` to avoid namespace pollution: \
                             `{trait_name} as _`"
                        ),
                        sugg,
                        rustc_errors::Applicability::MachineApplicable,
                    );
                }
            }
        }
    }
}

trait FindTrait {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool;
}

impl<T: ?Sized + FindTrait> FindTrait for &T {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        (*self).find(trait_name, hir)
    }
}

impl FindTrait for Mod<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.item_ids
            .iter()
            .map(|item_id| hir.item(*item_id))
            .any(|item| item.find(trait_name, hir))
    }
}

impl FindTrait for Item<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self.kind {
            ItemKind::Mod(module) => module.find(trait_name, hir),
            ItemKind::Fn(sig, generics, body_id) => {
                sig.find(trait_name, hir)
                    || generics.find(trait_name, hir)
                    || hir.body(body_id).find(trait_name, hir)
            }
            ItemKind::Static(ty, _, body_id) => {
                ty.find(trait_name, hir) || hir.body(body_id).find(trait_name, hir)
            }
            ItemKind::Const(ty, generics, body_id) => {
                ty.find(trait_name, hir)
                    || generics.find(trait_name, hir)
                    || hir.body(body_id).find(trait_name, hir)
            }
            ItemKind::TyAlias(ty, generics) => {
                ty.find(trait_name, hir) || generics.find(trait_name, hir)
            }
            ItemKind::Enum(_, generics) | ItemKind::Union(_, generics) => {
                generics.find(trait_name, hir)
            }
            ItemKind::Struct(variant_data, generics) => {
                variant_data.find(trait_name, hir) || generics.find(trait_name, hir)
            }
            ItemKind::Trait(_, _, generics, generic_bounds, trait_item_refs) => {
                generics.find(trait_name, hir)
                    || generic_bounds.find(trait_name, hir)
                    || trait_item_refs
                        .iter()
                        .map(|trait_item_ref| hir.trait_item(trait_item_ref.id))
                        .any(|trait_item| trait_item.find(trait_name, hir))
            }
            ItemKind::TraitAlias(generics, generic_bounds) => {
                generics.find(trait_name, hir) || generic_bounds.find(trait_name, hir)
            }
            ItemKind::Impl(impl_statement) => {
                impl_statement
                    .items
                    .iter()
                    .map(|impl_item_ref| hir.impl_item(impl_item_ref.id))
                    .any(|impl_item| impl_item.find(trait_name, hir))
                    || impl_statement.generics.find(trait_name, hir)
                    || impl_statement
                        .of_trait
                        .as_ref()
                        .map_or(false, |trait_ref| trait_ref.find(trait_name, hir))
            }
            ItemKind::OpaqueTy(opaque_ty) => {
                opaque_ty.generics.find(trait_name, hir) || opaque_ty.bounds.find(trait_name, hir)
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
}

impl FindTrait for VariantData<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self {
            VariantData::Struct {
                fields, ..
            } => fields.iter().any(|field| field.ty.find(trait_name, hir)),
            VariantData::Tuple(fields, ..) => {
                fields.iter().any(|field| field.ty.find(trait_name, hir))
            }
            VariantData::Unit(..) => false,
        }
    }
}

impl FindTrait for Body<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.params
            .iter()
            .any(|param| param.pat.find(trait_name, hir))
            || self.value.find(trait_name, hir)
    }
}

impl FindTrait for TraitItem<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.generics.find(trait_name, hir)
            || match self.kind {
                TraitItemKind::Const(ty, _) => ty.find(trait_name, hir),
                TraitItemKind::Fn(fn_sig, trait_fn) => {
                    fn_sig.find(trait_name, hir) || trait_fn.find(trait_name, hir)
                }
                TraitItemKind::Type(bounds, ty) => {
                    bounds.find(trait_name, hir) || ty.map_or(false, |ty| ty.find(trait_name, hir))
                }
            }
    }
}

impl FindTrait for TraitFn<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self {
            TraitFn::Required(idents) => idents.iter().any(|ident| ident.name == trait_name),
            TraitFn::Provided(body_id) => hir.body(*body_id).find(trait_name, hir),
        }
    }
}

impl FindTrait for ImplItem<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.generics.find(trait_name, hir)
            || match self.kind {
                ImplItemKind::Const(ty, _) | ImplItemKind::Type(ty) => ty.find(trait_name, hir),
                ImplItemKind::Fn(fn_sig, body_id) => {
                    fn_sig.find(trait_name, hir) || hir.body(body_id).find(trait_name, hir)
                }
            }
    }
}

impl FindTrait for FnSig<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.decl.find(trait_name, hir)
    }
}

impl FindTrait for FnDecl<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.inputs.iter().any(|input| input.find(trait_name, hir))
            || match self.output {
                FnRetTy::Return(ty) => ty.find(trait_name, hir),
                FnRetTy::DefaultReturn(_) => false,
            }
    }
}

impl FindTrait for Block<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.stmts.iter().any(|stmt| stmt.find(trait_name, hir))
            || self.expr.map_or(false, |expr| expr.find(trait_name, hir))
    }
}

impl FindTrait for Stmt<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self.kind {
            StmtKind::Let(let_stmt) => let_stmt.find(trait_name, hir),
            StmtKind::Item(item) => hir.item(item).find(trait_name, hir),
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => expr.find(trait_name, hir),
        }
    }
}

impl FindTrait for Expr<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self.kind {
            ExprKind::Path(qpath) => qpath.find(trait_name, hir),
            ExprKind::Call(expr, exprs) => {
                expr.find(trait_name, hir) || exprs.iter().any(|expr| expr.find(trait_name, hir))
            }
            ExprKind::MethodCall(path_segment, expr, exprs, _) => {
                path_segment.find(trait_name, hir)
                    || expr.find(trait_name, hir)
                    || exprs.iter().any(|expr| expr.find(trait_name, hir))
            }
            ExprKind::Struct(qpath, fields, _) => {
                qpath.find(trait_name, hir)
                    || fields.iter().any(|field| field.expr.find(trait_name, hir))
            }
            ExprKind::Field(expr, _) => expr.find(trait_name, hir),
            ExprKind::Cast(expr, ty) | ExprKind::Type(expr, ty) => {
                expr.find(trait_name, hir) || ty.find(trait_name, hir)
            }
            ExprKind::Block(block, _) => block.find(trait_name, hir),
            ExprKind::Closure(closure) => closure.find(trait_name, hir),
            ExprKind::DropTemps(expr) => expr.find(trait_name, hir),
            ExprKind::AddrOf(_, _, expr) => expr.find(trait_name, hir),
            ExprKind::Tup(exprs) | ExprKind::Array(exprs) => {
                exprs.iter().any(|expr| expr.find(trait_name, hir))
            }
            ExprKind::If(cond, then, els) => {
                cond.find(trait_name, hir)
                    || then.find(trait_name, hir)
                    || els.map_or(false, |els| els.find(trait_name, hir))
            }
            ExprKind::Match(expr, arms, _) => {
                expr.find(trait_name, hir) || arms.iter().any(|arm| arm.find(trait_name, hir))
            }
            _ => false,
        }
    }
}

impl FindTrait for Arm<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.pat.find(trait_name, hir)
            || self
                .guard
                .map_or(false, |guard| guard.find(trait_name, hir))
            || self.body.find(trait_name, hir)
    }
}

impl FindTrait for Closure<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.fn_decl.find(trait_name, hir)
            || self.bound_generic_params.find(trait_name, hir)
            || hir.body(self.body).find(trait_name, hir)
    }
}

impl FindTrait for LetStmt<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.ty.map_or(false, |ty| ty.find(trait_name, hir))
            || self.init.map_or(false, |expr| expr.find(trait_name, hir))
            || self.els.map_or(false, |els| els.find(trait_name, hir))
            || self.pat.find(trait_name, hir)
    }
}

impl FindTrait for Pat<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self.kind {
            PatKind::Path(qpath) => qpath.find(trait_name, hir),
            PatKind::Struct(qpath, fields, _) => {
                qpath.find(trait_name, hir)
                    || fields.iter().any(|field| field.pat.find(trait_name, hir))
            }
            PatKind::TupleStruct(qpath, pats, _) => {
                qpath.find(trait_name, hir) || pats.iter().any(|pat| pat.find(trait_name, hir))
            }
            PatKind::Or(pats) => pats.iter().any(|pat| pat.find(trait_name, hir)),
            PatKind::Tuple(pats, _) => pats.iter().any(|pat| pat.find(trait_name, hir)),
            PatKind::Box(pat) => pat.find(trait_name, hir),
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
}

impl FindTrait for QPath<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self {
            QPath::Resolved(ty, path) => {
                path.find(trait_name, hir) || ty.map_or(false, |ty| ty.find(trait_name, hir))
            }
            QPath::TypeRelative(ty, path_segment) => {
                ty.find(trait_name, hir) || path_segment.find(trait_name, hir)
            }
            QPath::LangItem(..) => false,
        }
    }
}

impl FindTrait for Generics<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.params.find(trait_name, hir)
            || self
                .predicates
                .iter()
                .any(|predicate| predicate.find(trait_name, hir))
    }
}

impl FindTrait for WherePredicate<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        if let WherePredicate::BoundPredicate(predicate) = self {
            predicate.bound_generic_params.find(trait_name, hir)
                || predicate.bounded_ty.find(trait_name, hir)
                || predicate.bounds.find(trait_name, hir)
        } else {
            false
        }
    }
}

impl FindTrait for Ty<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self.kind {
            TyKind::TraitObject(trait_refs, ..) => trait_refs
                .iter()
                .any(|(poly_trait_ref, _)| poly_trait_ref.find(trait_name, hir)),
            TyKind::OpaqueDef(item_id, ..) => hir.item(item_id).find(trait_name, hir),
            TyKind::Path(qpath) => qpath.find(trait_name, hir),
            TyKind::Slice(ty) | TyKind::Array(ty, _) => ty.find(trait_name, hir),
            TyKind::Ptr(mut_ty) | TyKind::Ref(_, mut_ty) => mut_ty.ty.find(trait_name, hir),
            TyKind::Tup(tys) => tys.iter().any(|ty| ty.find(trait_name, hir)),
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
}

impl FindTrait for [GenericBound<'_>] {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.iter().any(|bound| {
            if let GenericBound::Trait(poly_trait_ref, _modifier) = bound {
                poly_trait_ref.find(trait_name, hir)
            } else {
                false
            }
        })
    }
}

impl FindTrait for [GenericParam<'_>] {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.iter()
            .any(|generic_param| generic_param.find(trait_name, hir))
    }
}

impl FindTrait for GenericParam<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        match self.kind {
            GenericParamKind::Type {
                default,
                synthetic: _,
            } => default.map_or(false, |ty| ty.find(trait_name, hir)),
            GenericParamKind::Const {
                ty,
                default: _,
                is_host_effect: _,
                synthetic: _,
            } => ty.find(trait_name, hir),
            GenericParamKind::Lifetime {
                kind: _,
            } => false,
        }
    }
}

impl FindTrait for PolyTraitRef<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.bound_generic_params.find(trait_name, hir) || self.trait_ref.find(trait_name, hir)
    }
}

impl FindTrait for TraitRef<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.path.find(trait_name, hir)
    }
}

impl FindTrait for Path<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.find(trait_name, hir))
    }
}

impl FindTrait for GenericArgs<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.args.iter().any(|arg| {
            if let GenericArg::Type(ty) = arg {
                ty.find(trait_name, hir)
            } else {
                false
            }
        })
    }
}

impl FindTrait for PathSegment<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map) -> bool {
        self.args.map_or(false, |args| args.find(trait_name, hir)) || self.ident.name == trait_name
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
