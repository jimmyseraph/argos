
use syn::parse::{Parse, ParseStream};
use syn::{ Ident, Token, token, LitStr, Error, LitInt};

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct RouteAttribute {
    pub(crate) http_method: Ident,
    pub(crate) path: LitStr,
    pub(crate) formatter: LitStr,
}

impl Parse for RouteAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let methods = vec!["GET", "POST", "OPTIONS", "HEAD", "DELETE", "PUT", "TRACE", "CONNECT", "PATCH"];
        let http_method: Ident = input.parse()?;
        
        if methods.iter().find(|m| m.to_string() == http_method.to_string()).is_none() {
            return Err(Error::new(http_method.span(), "expected http method"));
        }
        let _comma = input.parse::<Token![,]>()?;
        let path_ident: Ident = input.parse()?;
        if path_ident != "path" || !input.peek(token::Eq) {
            return Err(Error::new(path_ident.span(), "expected path"));
        }
        let _eq = input.parse::<Token![=]>()?;
        let path = input.parse::<LitStr>()?;
        let _comma = input.parse::<Token![,]>()?;
        let formatter_ident: Ident = input.parse()?;
        if formatter_ident != "formatter" || !input.peek(token::Eq) {
            return Err(Error::new(formatter_ident.span(), "expected formatter"));
        }
        let _eq = input.parse::<Token![=]>()?;
        let formatter = input.parse::<LitStr>()?;
        let formatters = vec!["json", "text", "html"];
        if formatters.iter().find(|f| f.to_string() == formatter.value()).is_none() {
            return Err(Error::new(formatter.span(), "formatter only support json, text, html"));
        }
        Ok(RouteAttribute {
            http_method,
            path,
            formatter,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct FilterAttribute {
    pub(crate) predicate_type: Ident,
    pub(crate) predicate_value: LitStr,
    pub(crate) order: LitInt,
}


impl Parse for FilterAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let predicate_ident: Ident = input.parse()?;
        if predicate_ident == "path_pattern" && input.peek(token::Eq) {
            let _eq = input.parse::<Token![=]>()?;
            let path_pattern: LitStr = input.parse()?;
            let _comma = input.parse::<Token![,]>()?;
            let order_ident: Ident = input.parse()?;
            if order_ident == "order" && input.peek(token::Eq) {
                let _eq = input.parse::<Token![=]>()?;
                let order = input.parse::<LitInt>()?;
                Ok(FilterAttribute {
                    predicate_type: predicate_ident,
                    predicate_value: path_pattern,
                    order,
                })
            } else {
                Err(Error::new(order_ident.span(), "expected order"))
            }
        } else {
            Err(Error::new(predicate_ident.span(), "expected path_pattern"))
        }
    }
}