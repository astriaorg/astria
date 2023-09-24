use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input,
    Item,
};

#[proc_macro_attribute]
pub fn astria_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_ast = parse_macro_input!(item as Item);

    let Item::Struct(ref item_struct) = item_ast else {
        panic!("the astria_config proc macro can only be called on structs");
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

    // Check if log is a valid member of the config, otherwise panic
    let true = struct_fields
        .iter()
        .map(|x| x.0.to_string().to_lowercase())
        .collect::<Vec<_>>()
        .contains(&"log".to_string())
    else {
        panic!("config struct does not contain a log element, which is necessary");
    };

    // Check if the macro itself has a valid attribute
    let attr_str = attr.to_string().to_lowercase();
    match attr_str.as_str() {
        "conductor" | "composer" | "sequencer" | "sequencer_relayer" => {}
        _ => panic!(
            "Invalid attribute {attr_str}: please select a valid astria service to generate \
             config for"
        ),
    };

    let prefix = format!("ASTRIA_{}_", attr_str.to_uppercase());
    let test_prefix = format!("TESTTEST_{}", prefix);

    // let struct_field_ast = struct_fields
    //     .iter()
    //     .map(|(id, ty, vis)| {
    //         quote! {
    //             #vis #id: #ty
    //         }
    //     })
    //     .collect::<Vec<_>>();

    let code_gen_ast = quote! {
        const ENV_PREFIX: &str = #prefix;

        use serde::{Serialize, Deserialize};

        // Adding serde dependencies on top of the old struct
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        #item_ast

        pub fn get() -> Result<#struct_name, figment::Error> {
            #struct_name::from_environment(ENV_PREFIX)
        }

        impl #struct_name {
            fn from_environment(env_prefix: &str) -> Result<Config, figment::Error> {
                figment::Figment::new()
                    .merge(figment::providers::Env::prefixed("RUST_").split("_").only(&["log"]))
                    .merge(figment::providers::Env::prefixed(env_prefix))
                    .extract()
            }
        }

        #[cfg(test)]
        mod tests {
            use super::{#struct_name, ENV_PREFIX};
            const EXAMPLE_ENV: &str = include_str!("../local.env.example");

            fn populate_environment_from_example(jail: &mut figment::Jail, test_envar_prefix: &str) {
                static RE_START: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| regex::Regex::new(r"^[[:space:]]+").unwrap());
                static RE_END: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| regex::Regex::new(r"[[:space:]]+$").unwrap());
                for line in EXAMPLE_ENV.lines() {
                    if let Some((key, val)) = line.trim().split_once('=') {
                        assert!(
                            !(RE_END.is_match(key) || RE_START.is_match(val)),
                            "env vars must not contain spaces in assignment\n{line}"
                        );
                        let prefixed_key = format!("{test_envar_prefix}_{key}");
                        dbg!(&prefixed_key);
                        dbg!(&val);
                        jail.set_env(prefixed_key, val);
                    }
                }
            }

            #[test]
            fn ensure_example_env_is_in_sync() {
                figment::Jail::expect_with(|jail| {
                    populate_environment_from_example(jail, "TESTTEST");
                    #struct_name::from_environment(#test_prefix).unwrap();
                    Ok(())
                });
            }

            #[test]
            #[should_panic]
            fn extra_env_vars_are_rejected() {
                figment::Jail::expect_with(|jail| {
                    populate_environment_from_example(jail, "TESTTEST");
                    let bad_prefix = format!("{}_FOOBAR", #test_prefix);
                    jail.set_env(bad_prefix, "BAZ");
                    #struct_name::from_environment(#test_prefix).unwrap();
                    Ok(())
                });
            }
        }

    };

    code_gen_ast.into()
}
