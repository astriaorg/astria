#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::collections::HashMap;

use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_hir::{
    def::{
        DefKind,
        Res,
    },
    def_id::DefId,
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
    UsePath,
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
        if is_generated_file(cx, item.span) {
            return;
        }

        let hir_map = cx.tcx.hir();
        if let ItemKind::Use(path, UseKind::Single) = item.kind {
            // Check if the import is a trait
            if let Res::Def(DefKind::Trait, _) = path.res[0] {
                let parent_def_id = hir_map.get_parent_item(item.hir_id()).to_def_id();
                let parent = match hir_map.get_if_local(parent_def_id.into()) {
                    Some(node) => node,
                    None => {
                        log_error(cx, "parent of `Use` item not local".to_string());
                        return;
                    }
                };

                // Check if this is a re-export - ignore if it is
                if is_reexport(cx, item, parent_def_id, parent) {
                    return;
                }

                // Extract trait information and check if it's already aliased
                let (trait_name, span_snippet) = match extract_trait_info(cx, item, path) {
                    Ok(Some((name, snippet))) => (name, snippet),
                    Ok(None) => return, // Trait already aliased `as _`
                    Err(err) => {
                        log_error(cx, err);
                        return;
                    }
                };

                // Create cache
                let mut cache = CheckedImportCache::new();

                // Check if the trait name is explicitly used in parent scope
                let found = match parent {
                    Node::Crate(module) => {
                        cache.find_with_cache(module, trait_name, parent_def_id, &hir_map)
                    }
                    Node::Item(item) => {
                        cache.find_with_cache(item, trait_name, parent_def_id, &hir_map)
                    }
                    Node::Block(block) => {
                        cache.find_with_cache(block, trait_name, parent_def_id, &hir_map)
                    }
                    Node::Stmt(stmt) => {
                        cache.find_with_cache(stmt, trait_name, parent_def_id, &hir_map)
                    }
                    Node::TraitItem(trait_item) => {
                        cache.find_with_cache(trait_item, trait_name, parent_def_id, &hir_map)
                    }
                    Node::ImplItem(impl_item) => {
                        cache.find_with_cache(impl_item, trait_name, parent_def_id, &hir_map)
                    }
                    _ => {
                        log_error(cx, "unexpected parent of `Use` item".to_string());
                        return;
                    }
                };

                // Check if the trait name is explicitly used
                if !found {
                    suggest_alias(cx, item, trait_name, span_snippet);
                }
            }
        }
    }
}

#[derive(Default)]
pub struct CheckedImportCache {
    // Cache for FindTrait results to avoid redundant traversals
    cache: HashMap<(Symbol, DefId), bool>,
}

impl CheckedImportCache {
    fn new() -> Self {
        CheckedImportCache {
            cache: HashMap::new(),
        }
    }

    fn find_with_cache<T: ?Sized + FindTrait>(
        &mut self,
        item: &T,
        trait_name: Symbol,
        def_id: DefId,
        hir: &Map,
    ) -> bool {
        // Check if result is already in cache
        if let Some(&result) = self.cache.get(&(trait_name, def_id)) {
            return result;
        }

        // Compute and cache the result
        let result = item.find(trait_name, hir, self);
        self.cache.insert((trait_name, def_id), result);
        result
    }
}

fn log_error(cx: &LateContext<'_>, msg: String) {
    cx.sess().dcx().warn(msg);
}

// Checks if a file is generated
fn is_generated_file(cx: &LateContext<'_>, span: rustc_span::Span) -> bool {
    let file_name = cx.sess().source_map().span_to_filename(span);
    if let FileName::Real(path) = file_name {
        path.to_string_lossy(FileNameDisplayPreference::Local)
            .contains("crates/astria-core/src/generated")
    } else {
        false
    }
}

// Checks if import is a re-export
fn is_reexport<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
    parent_def_id: DefId,
    parent: Node<'tcx>,
) -> bool {
    // If the parent is a function, the trait cannot be a re-export and the visibility
    // check is skipped
    if matches!(
        parent,
        Node::Item(Item {
            kind: ItemKind::Fn(..),
            ..
        })
    ) {
        return false;
    }

    // Check if this is a re-export
    let visibility = cx.tcx.visibility(item.owner_id.to_def_id());
    match visibility {
        Visibility::Restricted(restricted_id) => {
            cx.tcx.def_path_hash(restricted_id) != cx.tcx.def_path_hash(parent_def_id)
        }
        _ => true, // If the visibility is public, this is a re-export
    }
}

// Extract trait name and span snippet
fn extract_trait_info<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx Item<'tcx>,
    path: &'tcx UsePath<'tcx>,
) -> Result<Option<(Symbol, String)>, String> {
    // Get the snippet for the import
    let span_snippet = match cx.sess().source_map().span_to_snippet(item.span) {
        Ok(snippet) => snippet,
        Err(err) => {
            return Err(format!(
                "Failed to extract source text from `Use` item at {:?}.\n
                Error: {:?}",
                item.span, err
            ));
        }
    };

    // If trait is already aliased `as _`, ignore
    if span_snippet.contains(" as _") {
        return Ok(None);
    }

    // Extract trait name
    let trait_name = match path.segments.last() {
        Some(segment) if segment.ident.name != kw::Underscore => segment.ident.name,
        Some(_) => return Ok(None),
        None => {
            return Err(format!(
                "Failed to extract trait name from `Use` item at {:?}.",
                item.span
            ));
        }
    };

    // If the trait is aliased as something other than `_`, assign `trait_name` to alias
    // and trim the alias from the span snippet for correct lint suggestion
    let mut result_trait_name = trait_name;
    let mut result_snippet = span_snippet.clone();

    if trait_name != item.ident.name && span_snippet.contains(" as ") {
        let mut semicolon_suffix = false;
        let mut suffix_to_strip = format!(" as {}", item.ident.name);
        if span_snippet.ends_with(';') {
            semicolon_suffix = true;
            suffix_to_strip.push(';');
        }

        if let Some(stripped) = span_snippet.strip_suffix(&suffix_to_strip) {
            result_snippet = stripped.to_string();
            result_trait_name = item.ident.name;
            if semicolon_suffix {
                result_snippet.push(';');
            }
        } else {
            return Err(format!(
                "Failed to strip suffix '{}' from '{}'",
                suffix_to_strip, span_snippet
            ));
        }
    }

    Ok(Some((result_trait_name, result_snippet)))
}

// Create and provide suggestion
fn suggest_alias(cx: &LateContext<'_>, item: &Item<'_>, trait_name: Symbol, span_snippet: String) {
    // Semicolons are included in the span snippet, so they must be handled accordingly
    let sugg = if span_snippet.ends_with(';') {
        format!("{} as _;", span_snippet.strip_suffix(';').unwrap())
    } else {
        // Commas are not included in span snippet
        format!("{span_snippet} as _")
    };

    span_lint_and_sugg(
        cx,
        ALIAS_TRAIT_IMPORTS,
        item.span,
        format!(
            "Trait `{trait_name}` is imported but not explicitly used. Consider `{trait_name} as \
             _`.",
        ),
        format!(
            "Consider importing the trait `as _` to avoid namespace pollution: `{trait_name} as _`"
        ),
        sugg,
        rustc_errors::Applicability::MachineApplicable,
    );
}

trait FindTrait {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool;
}

impl<T: ?Sized + FindTrait> FindTrait for &T {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        (*self).find(trait_name, hir, cache)
    }
}

impl FindTrait for Mod<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.item_ids.iter().any(|item_id| {
            let item = hir.item(*item_id);
            let def_id = item.owner_id.to_def_id();

            // Check if in cache first
            if let Some(&result) = cache.cache.get(&(trait_name, def_id)) {
                return result;
            }

            let result = item.find(trait_name, hir, cache);
            cache.cache.insert((trait_name, def_id), result);
            result
        })
    }
}

impl FindTrait for Item<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        // Return cached result if available
        let def_id = self.owner_id.to_def_id();
        if let Some(&result) = cache.cache.get(&(trait_name, def_id)) {
            return result;
        }

        use ItemKind::*;
        let result = match self.kind {
            Mod(module) => module.find(trait_name, hir, cache),
            Fn(sig, generics, body_id) => {
                sig.find(trait_name, hir, cache)
                    || generics.find(trait_name, hir, cache)
                    || hir.body(body_id).find(trait_name, hir, cache)
            }
            Static(ty, _, body_id) => {
                ty.find(trait_name, hir, cache) || hir.body(body_id).find(trait_name, hir, cache)
            }
            Const(ty, generics, body_id) => {
                ty.find(trait_name, hir, cache)
                    || generics.find(trait_name, hir, cache)
                    || hir.body(body_id).find(trait_name, hir, cache)
            }
            TyAlias(ty, generics) => {
                ty.find(trait_name, hir, cache) || generics.find(trait_name, hir, cache)
            }
            Enum(_, generics) | Union(_, generics) => generics.find(trait_name, hir, cache),
            Struct(variant_data, generics) => {
                variant_data.find(trait_name, hir, cache) || generics.find(trait_name, hir, cache)
            }
            Trait(_, _, generics, generic_bounds, trait_item_refs) => {
                generics.find(trait_name, hir, cache)
                    || generic_bounds.find(trait_name, hir, cache)
                    || trait_item_refs.iter().any(|trait_item_ref| {
                        let trait_item = hir.trait_item(trait_item_ref.id);
                        let trait_item_def_id = trait_item.owner_id.to_def_id();

                        if let Some(&result) = cache.cache.get(&(trait_name, trait_item_def_id)) {
                            return result;
                        }

                        let result = trait_item.find(trait_name, hir, cache);
                        cache.cache.insert((trait_name, trait_item_def_id), result);
                        result
                    })
            }
            TraitAlias(generics, generic_bounds) => {
                generics.find(trait_name, hir, cache) || generic_bounds.find(trait_name, hir, cache)
            }
            Impl(impl_statement) => {
                impl_statement.items.iter().any(|impl_item_ref| {
                    let impl_item = hir.impl_item(impl_item_ref.id);
                    let impl_item_def_id = impl_item.owner_id.to_def_id();

                    if let Some(&result) = cache.cache.get(&(trait_name, impl_item_def_id)) {
                        return result;
                    }

                    let result = impl_item.find(trait_name, hir, cache);
                    cache.cache.insert((trait_name, impl_item_def_id), result);
                    result
                }) || impl_statement.generics.find(trait_name, hir, cache)
                    || impl_statement
                        .of_trait
                        .as_ref()
                        .map_or(false, |trait_ref| trait_ref.find(trait_name, hir, cache))
            }
            OpaqueTy(opaque_ty) => {
                opaque_ty.generics.find(trait_name, hir, cache)
                    || opaque_ty.bounds.find(trait_name, hir, cache)
            }
            ExternCrate(_)
            | Use(..)
            | Macro(..)
            | ForeignMod {
                abi: _,
                items: _,
            }
            | GlobalAsm(_) => false,
        };

        // Cache and return result
        cache.cache.insert((trait_name, def_id), result);
        result
    }
}

impl FindTrait for VariantData<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use VariantData::*;
        match self {
            Struct {
                fields, ..
            } => fields
                .iter()
                .any(|field| field.ty.find(trait_name, hir, cache)),
            Tuple(fields, ..) => fields
                .iter()
                .any(|field| field.ty.find(trait_name, hir, cache)),
            Unit(..) => false,
        }
    }
}

impl FindTrait for Body<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.params
            .iter()
            .any(|param| param.pat.find(trait_name, hir, cache))
            || self.value.find(trait_name, hir, cache)
    }
}

impl FindTrait for TraitItem<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        // Check cache first
        let def_id = self.owner_id.to_def_id();
        if let Some(&result) = cache.cache.get(&(trait_name, def_id)) {
            return result;
        }

        use TraitItemKind::*;
        let result = self.generics.find(trait_name, hir, cache)
            || match self.kind {
                Const(ty, _) => ty.find(trait_name, hir, cache),
                Fn(fn_sig, trait_fn) => {
                    fn_sig.find(trait_name, hir, cache) || trait_fn.find(trait_name, hir, cache)
                }
                Type(bounds, ty) => {
                    bounds.find(trait_name, hir, cache)
                        || ty.map_or(false, |ty| ty.find(trait_name, hir, cache))
                }
            };

        // Cache result
        cache.cache.insert((trait_name, def_id), result);
        result
    }
}

impl FindTrait for TraitFn<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use TraitFn::*;
        match self {
            Required(idents) => idents.iter().any(|ident| ident.name == trait_name),
            Provided(body_id) => hir.body(*body_id).find(trait_name, hir, cache),
        }
    }
}

impl FindTrait for ImplItem<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        // Check cache first
        let def_id = self.owner_id.to_def_id();
        if let Some(&result) = cache.cache.get(&(trait_name, def_id)) {
            return result;
        }

        use ImplItemKind::*;
        let result = self.generics.find(trait_name, hir, cache)
            || match self.kind {
                Const(ty, _) | Type(ty) => ty.find(trait_name, hir, cache),
                Fn(fn_sig, body_id) => {
                    fn_sig.find(trait_name, hir, cache)
                        || hir.body(body_id).find(trait_name, hir, cache)
                }
            };

        // Cache result
        cache.cache.insert((trait_name, def_id), result);
        result
    }
}

impl FindTrait for FnSig<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.decl.find(trait_name, hir, cache)
    }
}

impl FindTrait for FnDecl<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use FnRetTy::*;
        self.inputs
            .iter()
            .any(|input| input.find(trait_name, hir, cache))
            || match self.output {
                Return(ty) => ty.find(trait_name, hir, cache),
                DefaultReturn(_) => false,
            }
    }
}

impl FindTrait for Block<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.stmts
            .iter()
            .any(|stmt| stmt.find(trait_name, hir, cache))
            || self
                .expr
                .map_or(false, |expr| expr.find(trait_name, hir, cache))
    }
}

impl FindTrait for Stmt<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use StmtKind::*;
        match self.kind {
            Let(let_stmt) => let_stmt.find(trait_name, hir, cache),
            Item(item) => {
                let hir_item = hir.item(item);
                let def_id = hir_item.owner_id.to_def_id();

                if let Some(&result) = cache.cache.get(&(trait_name, def_id)) {
                    return result;
                }

                let result = hir_item.find(trait_name, hir, cache);
                cache.cache.insert((trait_name, def_id), result);
                result
            }
            Expr(expr) | Semi(expr) => expr.find(trait_name, hir, cache),
        }
    }
}

impl FindTrait for Expr<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use ExprKind::*;
        match self.kind {
            Path(qpath) => qpath.find(trait_name, hir, cache),
            Call(expr, exprs) => {
                expr.find(trait_name, hir, cache)
                    || exprs.iter().any(|expr| expr.find(trait_name, hir, cache))
            }
            MethodCall(path_segment, expr, exprs, _) => {
                path_segment.find(trait_name, hir, cache)
                    || expr.find(trait_name, hir, cache)
                    || exprs.iter().any(|expr| expr.find(trait_name, hir, cache))
            }
            Struct(qpath, fields, _) => {
                qpath.find(trait_name, hir, cache)
                    || fields
                        .iter()
                        .any(|field| field.expr.find(trait_name, hir, cache))
            }
            Field(expr, _) => expr.find(trait_name, hir, cache),
            Cast(expr, ty) | Type(expr, ty) => {
                expr.find(trait_name, hir, cache) || ty.find(trait_name, hir, cache)
            }
            Block(block, _) => block.find(trait_name, hir, cache),
            Closure(closure) => closure.find(trait_name, hir, cache),
            DropTemps(expr) => expr.find(trait_name, hir, cache),
            AddrOf(_, _, expr) => expr.find(trait_name, hir, cache),
            Tup(exprs) | Array(exprs) => exprs.iter().any(|expr| expr.find(trait_name, hir, cache)),
            If(cond, then, els) => {
                cond.find(trait_name, hir, cache)
                    || then.find(trait_name, hir, cache)
                    || els.map_or(false, |els| els.find(trait_name, hir, cache))
            }
            Match(expr, arms, _) => {
                expr.find(trait_name, hir, cache)
                    || arms.iter().any(|arm| arm.find(trait_name, hir, cache))
            }
            _ => false,
        }
    }
}

impl FindTrait for Arm<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.pat.find(trait_name, hir, cache)
            || self
                .guard
                .map_or(false, |guard| guard.find(trait_name, hir, cache))
            || self.body.find(trait_name, hir, cache)
    }
}

impl FindTrait for Closure<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.fn_decl.find(trait_name, hir, cache)
            || self.bound_generic_params.find(trait_name, hir, cache)
            || hir.body(self.body).find(trait_name, hir, cache)
    }
}

impl FindTrait for LetStmt<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.ty.map_or(false, |ty| ty.find(trait_name, hir, cache))
            || self
                .init
                .map_or(false, |expr| expr.find(trait_name, hir, cache))
            || self
                .els
                .map_or(false, |els| els.find(trait_name, hir, cache))
            || self.pat.find(trait_name, hir, cache)
    }
}

impl FindTrait for Pat<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use PatKind::*;
        match self.kind {
            Path(qpath) => qpath.find(trait_name, hir, cache),
            Struct(qpath, fields, _) => {
                qpath.find(trait_name, hir, cache)
                    || fields
                        .iter()
                        .any(|field| field.pat.find(trait_name, hir, cache))
            }
            TupleStruct(qpath, pats, _) => {
                qpath.find(trait_name, hir, cache)
                    || pats.iter().any(|pat| pat.find(trait_name, hir, cache))
            }
            Or(pats) => pats.iter().any(|pat| pat.find(trait_name, hir, cache)),
            Tuple(pats, _) => pats.iter().any(|pat| pat.find(trait_name, hir, cache)),
            Box(pat) => pat.find(trait_name, hir, cache),
            Wild | Lit(_) | Range(..) | Binding(..) | Never | Ref(..) | Deref(_) | Slice(..)
            | Err(_) => false,
        }
    }
}

impl FindTrait for QPath<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use QPath::*;
        match self {
            Resolved(ty, path) => {
                path.find(trait_name, hir, cache)
                    || ty.map_or(false, |ty| ty.find(trait_name, hir, cache))
            }
            TypeRelative(ty, path_segment) => {
                ty.find(trait_name, hir, cache) || path_segment.find(trait_name, hir, cache)
            }
            LangItem(..) => false,
        }
    }
}

impl FindTrait for Generics<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.params.find(trait_name, hir, cache)
            || self
                .predicates
                .iter()
                .any(|predicate| predicate.find(trait_name, hir, cache))
    }
}

impl FindTrait for WherePredicate<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        if let WherePredicate::BoundPredicate(predicate) = self {
            predicate.bound_generic_params.find(trait_name, hir, cache)
                || predicate.bounded_ty.find(trait_name, hir, cache)
                || predicate.bounds.find(trait_name, hir, cache)
        } else {
            false
        }
    }
}

impl FindTrait for Ty<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use TyKind::*;
        match self.kind {
            TraitObject(trait_refs, ..) => trait_refs
                .iter()
                .any(|(poly_trait_ref, _)| poly_trait_ref.find(trait_name, hir, cache)),
            OpaqueDef(item_id, ..) => {
                let item = hir.item(item_id);
                let def_id = item.owner_id.to_def_id();

                if let Some(&result) = cache.cache.get(&(trait_name, def_id)) {
                    return result;
                }

                let result = item.find(trait_name, hir, cache);
                cache.cache.insert((trait_name, def_id), result);
                result
            }
            Path(qpath) => qpath.find(trait_name, hir, cache),
            Slice(ty) | Array(ty, _) => ty.find(trait_name, hir, cache),
            Ptr(mut_ty) | Ref(_, mut_ty) => mut_ty.ty.find(trait_name, hir, cache),
            Tup(tys) => tys.iter().any(|ty| ty.find(trait_name, hir, cache)),
            InferDelegation(..) | Never | AnonAdt(_) | Typeof(_) | Infer | Err(_) | BareFn(_)
            | Pat(..) => false,
        }
    }
}

impl FindTrait for [GenericBound<'_>] {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.iter().any(|bound| {
            if let GenericBound::Trait(poly_trait_ref, _modifier) = bound {
                poly_trait_ref.find(trait_name, hir, cache)
            } else {
                false
            }
        })
    }
}

impl FindTrait for [GenericParam<'_>] {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.iter()
            .any(|generic_param| generic_param.find(trait_name, hir, cache))
    }
}

impl FindTrait for GenericParam<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        use GenericParamKind::*;
        match self.kind {
            Type {
                default,
                synthetic: _,
            } => default.map_or(false, |ty| ty.find(trait_name, hir, cache)),
            Const {
                ty,
                default: _,
                is_host_effect: _,
                synthetic: _,
            } => ty.find(trait_name, hir, cache),
            Lifetime {
                kind: _,
            } => false,
        }
    }
}

impl FindTrait for PolyTraitRef<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.bound_generic_params.find(trait_name, hir, cache)
            || self.trait_ref.find(trait_name, hir, cache)
    }
}

impl FindTrait for TraitRef<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.path.find(trait_name, hir, cache)
    }
}

impl FindTrait for Path<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.find(trait_name, hir, cache))
    }
}

impl FindTrait for GenericArgs<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.args.iter().any(|arg| {
            if let GenericArg::Type(ty) = arg {
                ty.find(trait_name, hir, cache)
            } else {
                false
            }
        })
    }
}

impl FindTrait for PathSegment<'_> {
    fn find(&self, trait_name: Symbol, hir: &Map, cache: &mut CheckedImportCache) -> bool {
        self.args
            .map_or(false, |args| args.find(trait_name, hir, cache))
            || self.ident.name == trait_name
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
