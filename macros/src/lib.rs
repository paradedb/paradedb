use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::__private::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, FnArg, ItemFn, Pat};

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
    let mod_name = Ident::new(
        &format!("_{}", {
            let mut uuid = uuid::Uuid::new_v4().as_simple().to_string();
            uuid.truncate(6);
            uuid
        }),
        item_fn.span(),
    );

    let fn_name = &item_fn.sig.ident;
    let fn_name_decorated = Ident::new(&format!("{fn_name}_bfn"), fn_name.span());
    let args = &item_fn.sig.inputs.iter().collect::<Vec<_>>();
    let arg_names = args
        .iter()
        .map(|arg| match arg {
            FnArg::Receiver(_) => Ok(Ident::new("self", arg.span())),
            FnArg::Typed(ty) => match ty.pat.as_ref() {
                Pat::Ident(ident) => Ok(Ident::new(&ident.ident.to_string(), ident.span())),
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

    let code = quote::quote! {
        mod #mod_name {
            use pgrx::{default, pg_extern, AnyElement, AnyNumeric, PostgresEnum, Range};
            use crate::schema::AnyEnum;
            use super::super::RangeRelation;

            #(#attributes)*
            pub fn #fn_name_decorated(field: crate::api::FieldName, #(#args),*) -> crate::query::SearchQueryInput {
                crate::query::pdb_query::to_search_query_input(field, super::super::#fn_name(#(#arg_names),*))
            }
        }
    };

    Ok(code)
}

fn err(msg: &str) -> String {
    format!("`#[builder_fn]` requires that {msg}")
}
