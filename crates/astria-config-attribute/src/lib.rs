use itertools::Itertools;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Ident, Item};

fn extract_struct_properties(item_ast: &Item) -> syn::Result<syn::Ident> {
    let item_top_level_span = item_ast.span();
    let Item::Struct(ref item_struct) = item_ast else {
        return Err(syn::Error::new(
            item_top_level_span,
            "the astria_config proc macro can only be called on structs",
        ));
    };

    let struct_name = item_struct.ident.clone();

    // Generates an vec of token-tree like structures of the atomicity we care about
    let struct_fields: Vec<Ident> = item_struct
        .fields
        .iter()
        .map(|x| {
            x.ident.clone().ok_or(syn::Error::new(
                item_top_level_span,
                "Missing field identifier in struct",
            ))
        })
        .try_collect()?;

    // Check if log is a valid member of the config, otherwise throw compile error
    let true = struct_fields
        .iter()
        .map(|x| x.to_string().to_lowercase())
        .collect::<Vec<_>>()
        .contains(&"log".to_string())
    else {
        return Err(syn::Error::new(
            item_ast.span(),
            "config struct does not contain a log field, which is necessary",
        ));
    };

    Ok(struct_name)
}

#[proc_macro_attribute]
pub fn astria_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_ast = parse_macro_input!(item as Item);
    let attr_str = attr.to_string();

    let struct_name = match extract_struct_properties(&item_ast) {
        Ok(ident) => ident,
        Err(err) => return err.to_compile_error().into(),
    };

    let code_gen_ast = quote! {
        #item_ast

        impl config::Config for #struct_name {
            const PREFIX: &'static str = #attr_str;
        }
    };

    code_gen_ast.into()
}
