// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::__private::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, FnArg, ItemFn, Pat};

/// A macro that transforms search-related functions into builder functions for ParadeDB's search functionality.
///
/// This macro is used in pg_search to generate builder functions that convert simple non-fielded search
/// functions into ones that return a `SearchQueryInput`, which requires a `FieldName`.
///
/// It must be used in conjunction with `#[pg_extern]`.
///
/// # Example
///
/// ```rust,no_run,compile_fail
/// #[builder_fn]
/// #[pg_extern(name = "foo_bar")]  // name= is required here
/// fn foo_bar(input: String) -> pdb::Query {
///     pdb::Query::FooBar { input }
/// }
/// ```
///
/// This will generate an additional function `match_query_bfn` that takes a field name as the first
/// parameter and converts the Query result into a SearchQueryInput for ParadeDB's search system.
///
/// That generated function will look like:
///
/// ```rust,no_run,compile_fail
/// #[pg_extern]
/// fn foo_bar(field: FieldName, input: String) -> SearchQueryInput {
///     to_search_query_input(field, foo_bar(input))
/// }
/// ```
///
/// From the SQL side of things, the programmer makes `foo_bar(input: String)` and it can be called
/// as
///
/// ```sql
/// SELECT * FROM t WHERE field @@@ pdb.foo_bar('hi mom');
/// ```
///
/// And this macro generates the fielded version, which allows pg_search (or the user) to write the
/// same query as:
///
/// ```sql
/// SELECT * FROM t WHERE key_field @@@ paradedb.foo_bar('field', 'hi mom');
/// ```
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
