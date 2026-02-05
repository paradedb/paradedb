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
    let schema = args.take_ident("schema").unwrap();
    let json_cast_name = args.take_ident("json_cast_name").unwrap();
    let jsonb_cast_name = args.take_ident("jsonb_cast_name").unwrap();
    let uuid_cast_name = args.take_ident("uuid_cast_name").unwrap();
    let text_array_cast_name = args.take_ident("text_array_cast_name").unwrap();
    let varchar_array_cast_name = args.take_ident("varchar_array_cast_name").unwrap();
    let pgrx_name = format!("{}_definition", sql_name.value());

    let create_type_sql = format!(
        r#"
            CREATE TYPE {schema}.{sql_name};

            CREATE OR REPLACE FUNCTION {schema}.{sql_name}_in(cstring) RETURNS {schema}.{sql_name} AS 'textin' LANGUAGE internal IMMUTABLE STRICT;
            CREATE OR REPLACE FUNCTION {schema}.{sql_name}_out({schema}.{sql_name}) RETURNS cstring AS 'textout' LANGUAGE internal IMMUTABLE STRICT;
            CREATE OR REPLACE FUNCTION {schema}.{sql_name}_send({schema}.{sql_name}) RETURNS bytea AS 'textsend' LANGUAGE internal IMMUTABLE STRICT;
            CREATE OR REPLACE FUNCTION {schema}.{sql_name}_recv(internal) RETURNS {schema}.{sql_name} AS 'textrecv' LANGUAGE internal IMMUTABLE STRICT;

            CREATE TYPE {schema}.{sql_name} (
                INPUT = {schema}.{sql_name}_in,
                OUTPUT = {schema}.{sql_name}_out,
                SEND = {schema}.{sql_name}_send,
                RECEIVE = {schema}.{sql_name}_recv,
                COLLATABLE = true,
                CATEGORY = 't', -- 't' is for tokenizer
                PREFERRED = {preferred},
                LIKE = text
            );
         "#,
        sql_name = sql_name.value(),
        preferred = preferred.value()
    );

    let pgrx_cast_to_text_array_name = format!("{}_cast_to_text_array", sql_name.value());
    let create_cast_to_text_array = format!(
        "CREATE CAST ({schema}.{sql_name} AS TEXT[]) WITH FUNCTION {schema}.{cast_name} AS IMPLICIT;",
        sql_name = sql_name.value(),
        cast_name = cast_name
    );

    let pgrx_cast_from_json_name = format!("{}_cast_from_json", sql_name.value());
    let create_cast_from_json = format!(
        r#"
        CREATE CAST (json AS {schema}.{sql_name}) WITH FUNCTION {schema}.{json_cast_name} AS ASSIGNMENT;
        CREATE CAST (jsonb AS {schema}.{sql_name}) WITH FUNCTION {schema}.{jsonb_cast_name} AS ASSIGNMENT;
        "#,
        sql_name = sql_name.value()
    );

    let pgrx_cast_from_uuid_name = format!("{}_cast_from_uuid", sql_name.value());
    let create_cast_from_uuid = format!(
        "CREATE CAST (uuid AS {schema}.{sql_name}) WITH FUNCTION {schema}.{uuid_cast_name} AS ASSIGNMENT;",
        sql_name = sql_name.value()
    );

    let pgrx_cast_from_text_array_name = format!("{}_cast_from_text_array", sql_name.value());
    let create_cast_from_text_array = format!(
        r#"
        CREATE CAST (text[] AS {schema}.{sql_name}) WITH FUNCTION {schema}.{text_array_cast_name} AS ASSIGNMENT;
        CREATE CAST (varchar[] AS {schema}.{sql_name}) WITH FUNCTION {schema}.{varchar_array_cast_name} AS ASSIGNMENT;
        "#,
        schema = schema.to_string(),
        sql_name = sql_name.value(),
    );

    let pgrx_cast_from_text_name = format!("{}_cast_from_text", sql_name.value());
    let create_cast_from_text = format!(
        r#"
        CREATE CAST (text AS {schema}.{sql_name}) WITH INOUT AS IMPLICIT;
        CREATE CAST (varchar AS {schema}.{sql_name}) WITH INOUT AS IMPLICIT;
        "#,
        schema = schema.to_string(),
        sql_name = sql_name.value(),
    );

    let typmod = if !custom_typmod {
        let alter_type_sql = format!(
            "ALTER TYPE {schema}.{sql_name} SET (TYPMOD_IN = generic_typmod_in, TYPMOD_OUT = generic_typmod_out);",
            sql_name = sql_name.value()
        );
        let alter_type_pgrx_name = format!("{}_alter_type", sql_name.value());
        quote! {
            extension_sql!(#alter_type_sql, name = #alter_type_pgrx_name, requires = [generic_typmod_in, generic_typmod_out, #pgrx_name]);
        }
    } else {
        quote! {}
    };

    let text_cast_sql = if sql_name.value() == "alias" {
        quote! {}
    } else {
        quote! {
            extension_sql!(#create_cast_from_text, name = #pgrx_cast_from_text_name, requires = [#pgrx_name]);
        }
    };

    quote! {
        extension_sql!(
            #create_type_sql,
            name = #pgrx_name,
            creates = [Type(#rust_name)],
        );

        #typmod

        extension_sql!(#create_cast_to_text_array, name = #pgrx_cast_to_text_array_name, requires = [#pgrx_name, #cast_name]);
        extension_sql!(#create_cast_from_json, name = #pgrx_cast_from_json_name, requires = [#pgrx_name, #json_cast_name, #jsonb_cast_name]);
        extension_sql!(#create_cast_from_uuid, name = #pgrx_cast_from_uuid_name, requires = [#pgrx_name, #uuid_cast_name]);
        extension_sql!(#create_cast_from_text_array, name = #pgrx_cast_from_text_array_name, requires = [#pgrx_name, #text_array_cast_name, #varchar_array_cast_name]);
        #text_cast_sql
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
