use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::__private::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, Attribute, Expr, FnArg, ItemFn, Meta, MetaList, MetaNameValue, Pat, PatType,
    Type,
};

#[proc_macro_attribute]
pub fn builder_fn(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut stream = proc_macro2::TokenStream::new();

    let item_fn = parse_macro_input!(item as syn::ItemFn);
    stream.extend(item_fn.to_token_stream());

    stream.extend(
        build_function(&item_fn).unwrap_or_else(|e| e.to_compile_error().to_token_stream()),
    );

    stream.into()
}

fn build_function(item_fn: &ItemFn) -> Result<proc_macro2::TokenStream, syn::Error> {
    // validate_args(item_fn)?;

    let mod_name = proc_macro2::Ident::new(
        &format!("_{}", uuid::Uuid::new_v4().as_simple().to_string()),
        item_fn.span(),
    );

    let fn_name = &item_fn.sig.ident;
    let fn_name_decorated = Ident::new(&format!("{}_bfn", fn_name), fn_name.span());
    let args = &item_fn.sig.inputs.iter().collect::<Vec<_>>();
    let arg_names = args
        .iter()
        .map(|arg| match arg {
            FnArg::Receiver(_) => Ok(proc_macro2::Ident::new("self", arg.span())),
            FnArg::Typed(ty) => match ty.pat.as_ref() {
                Pat::Ident(ident) => Ok(proc_macro2::Ident::new(
                    &ident.ident.to_string(),
                    ident.span(),
                )),
                _ => Err(syn::Error::new(
                    ty.span(),
                    "unsupported argument formulation",
                )),
            },
        })
        .collect::<Result<Vec<_>, syn::Error>>()?;
    let attributes = item_fn.attrs.iter().collect::<Vec<_>>();

    if attributes.is_empty() {
        return Err(syn::Error::new(
            item_fn.span(),
            err("`#[builder_fn]` be attached to a function prior to the `#[pg_extern]` definition"),
        ));
    }

    let mut sql_name = item_fn.sig.ident.to_string();
    for att in &attributes {
        let s = quote::quote! {#att}.to_string();
        let re = regex::Regex::new(r#"name\s*=\s*"(.*)""#).unwrap();
        if let Some(cap) = re.captures(&s) {
            sql_name = cap[1].to_string();
        }
    }

    dbg!(
        "sql_name: {}, attrs={}",
        &sql_name,
        quote::quote!(#(#attributes)*)
    );

    let custom_sql_name_begin = format!("{mod_name}_{fn_name_decorated}_begin");
    let custom_sql_name_middle = format!("{mod_name}_{fn_name_decorated}_middle");
    let custom_sql_name_end = format!("{mod_name}_{fn_name_decorated}_end");

    let code = quote::quote! {
        mod #mod_name {
            #[pgrx::pg_schema]
            mod paradedb_tmp {
                use pgrx::{default, pg_extern, AnyElement, AnyNumeric, PostgresEnum, Range};
                use crate::schema::AnyEnum;
                use super::super::RangeRelation;
                #(#attributes)*
                fn #fn_name_decorated(field: crate::api::FieldName, #(#args),*) -> crate::query::SearchQueryInput {
                    crate::query::fielded_query::to_search_query_input(field, super::super::#fn_name(#(#arg_names),*))
                }


                pgrx::extension_sql!("ALTER FUNCTION paradedb_tmp.", name = #custom_sql_name_begin, requires = [ #fn_name_decorated ]);
                pgrx::extension_sql!(#sql_name, name = #custom_sql_name_middle, requires = [ #custom_sql_name_begin ]);
                pgrx::extension_sql!(" SET SCHEMA paradedb;", name = #custom_sql_name_end, requires = [ #custom_sql_name_middle ]);
            }
        }
    };

    Ok(code.into())
}

fn err(msg: &str) -> String {
    format!("`#[builder_fn]` requires that {msg}")
}
