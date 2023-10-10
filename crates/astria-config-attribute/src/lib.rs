use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input,
    spanned::Spanned,
    Item,
};

fn extract_struct_properties(item_ast: &Item) -> syn::Result<syn::Ident> {
    let Item::Struct(ref item_struct) = item_ast else {
        return Err(syn::Error::new(
            item_ast.span(),
            "the astria_config proc macro can only be called on structs",
        ));
    };

    let struct_name = item_struct.ident.clone();

    // Generates an vec of token-tree like structures of the atomicity we care about
    let struct_fields = item_struct
        .fields
        .iter()
        .map(|x| {
            let field_ident = x.ident.clone().unwrap();
            let field_ty = x.ty.clone();
            let field_vis = x.vis.clone();

            (field_ident, field_ty, field_vis)
        })
        .collect::<Vec<_>>();

    // Check if log is a valid member of the config, otherwise throw compile error
    let true = struct_fields
        .iter()
        .map(|x| x.0.to_string().to_lowercase())
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

    let struct_name: syn::Ident = extract_struct_properties(&item_ast).unwrap();

    let code_gen_ast = quote! {
        #item_ast

        impl astria_config::AstriaConfig<'_> for #struct_name {
            const PREFIX: &'static str = #attr_str;
        }
    };

    code_gen_ast.into()
}
