use proc_macro::TokenStream;

use quote::quote;
use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Error, Ident, LitBool, LitStr, Result, Token};

pub fn generate_tokenizer_sql(input: TokenStream) -> TokenStream {
    let mut args = parse_macro_input!(input as NamedArgs);

    let rust_name = args.take_ident("rust_name").unwrap();
    let sql_name = args.take_str("sql_name").unwrap();
    let cast_name = args.take_ident("cast_name").unwrap();
    let preferred = args.take_bool("preferred").unwrap();
    let custom_typmod = args.take_bool("custom_typmod").unwrap().value();
    let pgrx_name = format!("{}_definition", sql_name.value());

    let create_type_sql = format!(
        r#"
            CREATE TYPE {sql_name};

            CREATE OR REPLACE FUNCTION {sql_name}_in(cstring) RETURNS {sql_name} AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
            CREATE OR REPLACE FUNCTION {sql_name}_out({sql_name}) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
            CREATE OR REPLACE FUNCTION {sql_name}_send({sql_name}) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
            CREATE OR REPLACE FUNCTION {sql_name}_recv(internal) RETURNS {sql_name} AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;

            CREATE TYPE {sql_name} (
                INPUT = {sql_name}_in,
                OUTPUT = {sql_name}_out,
                SEND = {sql_name}_send,
                RECEIVE = {sql_name}_recv,
                COLLATABLE = true,
                CATEGORY = 't', -- 't' for tokenizer
                PREFERRED = {preferred},
                LIKE = text
            );
         "#,
        sql_name = sql_name.value(),
        preferred = preferred.value(),
    );

    let pgrx_cast_name = format!("{}_cast", sql_name.value());
    let create_cast_sql = format!(
        "CREATE CAST ({sql_name} AS TEXT[]) WITH FUNCTION {cast_name} AS ASSIGNMENT;",
        sql_name = sql_name.value(),
        cast_name = cast_name.to_string()
    );

    let typmod = if !custom_typmod {
        let alter_type_sql = format!(
            "ALTER TYPE {} SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);",
            sql_name.value()
        );
        let alter_type_pgrx_name = format!("{}_alter_type", sql_name.value());
        quote! {
            extension_sql!(#alter_type_sql, name = #alter_type_pgrx_name, requires = [generic_typmod_in, generic_typmod_out, #pgrx_name]);
        }
    } else {
        quote! {}
    };

    quote! {
        extension_sql!(
            #create_type_sql,
            name = #pgrx_name,
            creates = [Type(#rust_name)],
        );

        #typmod

        extension_sql!(#create_cast_sql, name = #pgrx_cast_name, requires = [#pgrx_name, #cast_name]);
    }
        .into()
}

struct Kv {
    key: Ident,
    _eq: Token![=],
    value: Value,
}

enum Value {
    Ident(Ident),
    Str(LitStr),
    Bool(LitBool),
}

impl Parse for Kv {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;

        // For the 5 string fields we expect a string literal.
        // For `preferred` we expect a boolean literal.
        // We don't know which it is yet, so try both:
        // Prefer parsing a string; if that fails, try bool.
        // We'll validate type-by-key later.
        if input.peek(syn::Lit) {
            if input.peek(syn::LitStr) {
                let s: LitStr = input.parse()?;
                Ok(Kv {
                    key,
                    _eq,
                    value: Value::Str(s),
                })
            } else if input.peek(syn::LitBool) {
                let b: LitBool = input.parse()?;
                Ok(Kv {
                    key,
                    _eq,
                    value: Value::Bool(b),
                })
            } else {
                Err(Error::new(
                    input.span(),
                    "expected a string literal or boolean literal",
                ))
            }
        } else if input.peek(syn::Ident) {
            let i: Ident = input.parse()?;
            Ok(Kv {
                key,
                _eq,
                value: Value::Ident(i),
            })
        } else {
            Err(Error::new(input.span(), "expected a literal"))
        }
    }
}

struct NamedArgs {
    // store raw values so we can emit helpful errors
    map: HashMap<String, Value>,
}

impl Parse for NamedArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut map = HashMap::<String, Value>::new();

        // Parse `ident = <lit>` pairs separated by commas, trailing comma ok
        while !input.is_empty() {
            let kv: Kv = input.parse()?;
            let key = kv.key.to_string();

            if map.contains_key(&key) {
                return Err(Error::new(kv.key.span(), format!("duplicate key `{key}`")));
            }

            map.insert(key, kv.value);

            if input.is_empty() {
                break;
            }
            let _comma: Option<Token![,]> = input.parse()?;
        }

        Ok(Self { map })
    }
}

impl NamedArgs {
    fn take_ident(&mut self, key: &str) -> Result<Ident> {
        match self.map.remove(key) {
            Some(Value::Ident(i)) => Ok(i),
            Some(_) => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("`{key}` must be an identifier"),
            )),
            None => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("missing required key `{key}`"),
            )),
        }
    }
    fn take_str(&mut self, key: &str) -> Result<LitStr> {
        match self.map.remove(key) {
            Some(Value::Str(s)) => Ok(s),
            Some(_) => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("`{key}` must be a string literal"),
            )),
            None => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("missing required key `{key}`"),
            )),
        }
    }

    fn take_bool(&mut self, key: &str) -> Result<LitBool> {
        match self.map.remove(key) {
            Some(Value::Bool(b)) => Ok(b),
            Some(_) => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("`{key}` must be a boolean literal (true/false)"),
            )),
            None => Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("missing required key `{key}`"),
            )),
        }
    }
}
