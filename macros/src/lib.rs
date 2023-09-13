mod parse;
use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn, ReturnType};

#[doc(hidden)]
macro_rules! ctor_attributes {
    () => {
        // Linux/ELF: https://www.exploit-db.com/papers/13234

        // Mac details: https://blog.timac.org/2016/0716-constructor-and-destructor-attributes/

        // Why .CRT$XCU on Windows? https://www.cnblogs.com/sunkang/archive/2011/05/24/2055635.html
        // 'I'=C init, 'C'=C++ init, 'P'=Pre-terminators and 'T'=Terminators
        quote!(
            #[cfg_attr(any(target_os = "linux", target_os = "android"), link_section = ".init_array")]
            #[cfg_attr(target_os = "freebsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "netbsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "openbsd", link_section = ".init_array")]
            #[cfg_attr(target_os = "dragonfly", link_section = ".init_array")]
            #[cfg_attr(target_os = "illumos", link_section = ".init_array")]
            #[cfg_attr(target_os = "haiku", link_section = ".init_array")]
            #[cfg_attr(any(target_os = "macos", target_os = "ios"), link_section = "__DATA,__mod_init_func")]
            #[cfg_attr(windows, link_section = ".CRT$XCU")]
        )
    };
}

#[proc_macro_attribute]
pub fn register(_attribute: proc_macro::TokenStream, function: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item: syn::Item = syn::parse_macro_input!(function);
    if let syn::Item::Fn(function) = item {
        validate_item("ctor", &function);

        let syn::ItemFn {
            attrs,
            block,
            vis,
            sig:
                syn::Signature {
                    ident,
                    unsafety,
                    constness,
                    abi,
                    ..
                },
            ..
        } = function;

        let ctor_ident =
            syn::parse_str::<syn::Ident>(format!("{}___rust_ctor___ctor", ident).as_ref())
                .expect("Unable to create identifier");

        let tokens = ctor_attributes!();
        let output = quote!(
            #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd", target_os = "dragonfly", target_os = "illumos", target_os = "haiku", target_os = "macos", target_os = "ios", windows)))]
            compile_error!("#[register] is not supported on the current target");

            #(#attrs)*
            #vis #unsafety extern #abi #constness fn #ident() #block

            #[cfg_attr(not(feature = "used_linker"), used)]
            #[cfg_attr(feature = "used_linker", used(linker))]
            #[allow(non_upper_case_globals)]
            #[doc(hidden)]
            #tokens
            static #ctor_ident
            :
            unsafe extern "C" fn() -> usize =
            {
                #[cfg_attr(any(target_os = "linux", target_os = "android"), link_section = ".text.startup")]
                unsafe extern "C" fn #ctor_ident() -> usize { #ident(); 0 };
                #ctor_ident
            }
            ;
        );

        // eprintln!("{}", output);

        output.into()
    } else if let syn::Item::Static(var) = item {
        let syn::ItemStatic {
            ident,
            mutability,
            expr,
            attrs,
            ty,
            vis,
            ..
        } = var;

        if matches!(mutability, syn::StaticMutability::Mut(_)) {
            panic!("#[register]-annotated static objects must not be mutable");
        }

        if attrs.iter().any(|attr| {
            attr.path()
                .segments
                .iter()
                .any(|segment| segment.ident == "no_mangle")
        }) {
            panic!("#[register]-annotated static objects do not support #[no_mangle]");
        }

        let ctor_ident =
            syn::parse_str::<syn::Ident>(format!("{}___rust_ctor___ctor", ident).as_ref())
                .expect("Unable to create identifier");
        let storage_ident =
            syn::parse_str::<syn::Ident>(format!("{}___rust_ctor___storage", ident).as_ref())
                .expect("Unable to create identifier");

        let tokens = ctor_attributes!();
        let output = quote!(
            #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd", target_os = "dragonfly", target_os = "illumos", target_os = "haiku", target_os = "macos", target_os = "ios", windows)))]
            compile_error!("#[register] is not supported on the current target");

            // This is mutable, but only by this macro code!
            static mut #storage_ident: Option<#ty> = None;

            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #vis struct #ident<T> {
                _data: core::marker::PhantomData<T>
            }

            #(#attrs)*
            #vis static #ident: #ident<#ty> = #ident {
                _data: core::marker::PhantomData::<#ty>
            };

            impl core::ops::Deref for #ident<#ty> {
                type Target = #ty;
                fn deref(&self) -> &'static #ty {
                    unsafe {
                        #storage_ident.as_ref().unwrap()
                    }
                }
            }

            #[used]
            #[allow(non_upper_case_globals)]
            #tokens
            static #ctor_ident
            :
            unsafe extern "C" fn() = {
                #[cfg_attr(any(target_os = "linux", target_os = "android"), link_section = ".text.startup")]
                unsafe extern "C" fn initer() {
                    #storage_ident = Some(#expr);
                }; initer }
            ;
        );

        // eprintln!("{}", output);

        output.into()
    } else {
        panic!("#[register] items must be functions or static globals");
    }
}

#[proc_macro_attribute]
pub fn route(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {

    // parse function define
    let function = parse_macro_input!(input as ItemFn);

    // parse args
    let attribute = parse_macro_input!(args as parse::RouteAttribute);
    let http_method = attribute.http_method.to_string();
    let path = attribute.path;
    let formatter = attribute.formatter;

    // get the return type of the function
    let return_type = match &function.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => quote!(#ty),
    };
    // println!("{:?}", return_type);
    // rebuild new function
    let fn_name = &function.sig.ident;
    let fn_args = &function.sig.inputs;
    let fn_block = &function.block;

    let f = formatter.value();
    let handler = match f.as_str() {
        "json" => {
            quote! {
                pub fn #fn_name(#fn_args) -> 
                    std::pin::Pin<Box<dyn std::future::Future<Output = Result<argos::response::Response<argos::util::Full<argos::util::Bytes>>, argos::error::Error>> + Send>> {
                    Box::pin(async move {
                        let result: #return_type = #fn_block;
                        let response = match result {
                            Ok(data) => {
                                let response_body = serde_json::to_string(&data).unwrap();
                                argos::response::Response::builder()
                                    .header("Content-Type", "application/json")
                                    .body(argos::util::Full::new(argos::util::Bytes::from(response_body)))
                                    .unwrap()
                            },
                            Err(err) => {
                                let response_body = serde_json::to_string(&(err.response_body)).unwrap();
                                let mut builder = argos::response::Response::builder();
                                for (k, v) in err.headers.iter() {
                                    builder.headers_mut().unwrap().insert(k, v.clone());
                                }
                                builder
                                    .header("Content-Type", "application/json")
                                    .status(err.response_code)
                                    .body(argos::util::Full::new(argos::util::Bytes::from(response_body)))
                                    .unwrap()
                            },
                        };
                        return Ok(response);
                    })
                }
            }
        },
        "text" => {
            quote! {
                pub fn #fn_name(#fn_args) -> 
                    std::pin::Pin<Box<dyn std::future::Future<Output = Result<argos::response::Response<argos::util::Full<argos::util::Bytes>>, argos::error::Error>> + Send>> {
                    Box::pin(async move {
                        let result: #return_type = #fn_block;
                        let response = match result {
                            Ok(data) => {
                                let response_body = format!("{}", data);
                                argos::response::Response::builder()
                                    .header("Content-Type", "text/plain")
                                    .body(argos::util::Full::new(argos::util::Bytes::from(response_body)))
                                    .unwrap()
                            },
                            Err(err) => {
                                let response_body = format!("{}", err.response_body);
                                let mut builder = argos::response::Response::builder();
                                for (k, v) in err.headers.iter() {
                                    builder.headers_mut().unwrap().insert(k, v.clone());
                                }
                                builder
                                    .header("Content-Type", "text/plain")
                                    .status(err.response_code)
                                    .body(argos::util::Full::new(argos::util::Bytes::from(response_body)))
                                    .unwrap()
                            },
                        };
                        return Ok(response);
                    })
                }
            }
        },
        _ => {
            quote! {
                pub fn #fn_name(#fn_args) -> 
                    std::pin::Pin<Box<dyn std::future::Future<Output = Result<argos::response::Response<argos::util::Full<argos::util::Bytes>>, argos::error::Error>> + Send>> {
                    Box::pin(async move {
                        let result: #return_type = #fn_block;
                        let response = match result {
                            Ok(data) => {
                                let response_body = format!("{}", data);
                                argos::response::Response::builder()
                                    .header("Content-Type", "text/html")
                                    .body(argos::util::Full::new(argos::util::Bytes::from(response_body)))
                                    .unwrap()
                            },
                            Err(err) => {
                                let response_body = format!("{}", err.response_body);
                                let mut builder = argos::response::Response::builder();
                                for (k, v) in err.headers.iter() {
                                    builder.headers_mut().unwrap().insert(k, v.clone());
                                }
                                builder.header("Content-Type", "text/html")
                                    .status(err.response_code)
                                    .body(argos::util::Full::new(argos::util::Bytes::from(response_body)))
                                    .unwrap()
                            },
                        };
                        return Ok(response);
                    })
                }
            }
        }
    };


    let register_fn_name = format_ident!("register_{}", fn_name);
    let register_fn = quote! {
        #[register]
        fn #register_fn_name() {
            argos::ROUTE_TABLE.write().unwrap().push(
                argos::RouteInfo::new(
                    #http_method.to_string(),
                    argos::Path::new(#path),
                    Box::new(#fn_name)
                )
            );
        }
    };
    

    TokenStream::from(quote!{
        #handler
        #register_fn
    })
}

#[proc_macro_attribute]
pub fn filter(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // parse function define
    let function = parse_macro_input!(input as ItemFn);
    // parse args
    let attribute = parse_macro_input!(args as parse::FilterAttribute);
    let predicate_type = attribute.predicate_type.to_string();
    let predicate_value = attribute.predicate_value;
    let order = attribute.order;

    // get the return type of the function
    // get the return type of the function
    let return_type = match &function.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => quote!(#ty),
    };
  
    // rebuild new function
    let fn_name = &function.sig.ident;
    let fn_args = &function.sig.inputs;
    let fn_block = &function.block;

    let handler = quote! {
        pub fn #fn_name(#fn_args) -> std::pin::Pin<Box<dyn std::future::Future<Output = argos::Chain> + Send>> {
            Box::pin(
                async move {
                    let result: #return_type = #fn_block;
                    result
                }
            )
        }
    };

    let register_fn_name = format_ident!("register_{}", fn_name);
    let register_fn = quote! {
        #[register]
        fn #register_fn_name() {
            let mut table = argos::FILTER_TABLE.write().unwrap();
            table.push(
                argos::FilterInfo::new(
                    argos::Predicate::from_str(#predicate_type, #predicate_value),
                    #order,
                    Box::new(#fn_name)
                )
            );
            table.sort_by(|one, anther| one.order().cmp(&anther.order()));
        }
    };

    TokenStream::from(quote!{
        #handler
        #register_fn
    })
}

fn validate_item(typ: &str, item: &syn::ItemFn) {
    let syn::ItemFn { vis, sig, .. } = item;

    // Ensure that visibility modifier is not present
    match vis {
        syn::Visibility::Inherited => {}
        _ => panic!("#[{}] methods must not have visibility modifiers", typ),
    }

    // No parameters allowed
    if !sig.inputs.is_empty() {
        panic!("#[{}] methods may not have parameters", typ);
    }

    // No return type allowed
    match sig.output {
        syn::ReturnType::Default => {}
        _ => panic!("#[{}] methods must not have return types", typ),
    }
}