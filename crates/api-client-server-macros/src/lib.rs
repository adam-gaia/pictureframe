use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Attribute, FnArg, ImplItem, ImplItemFn, ItemImpl, Pat, PatType, ReturnType,
    Type,
};

/// Parsed representation of `#[api_handler(method = "POST", path = "/users/{id}")]`
#[derive(Debug, FromMeta)]
struct ApiHandlerArgs {
    method: String,
    path: String,
}

/// Parsed parameter info extracted from a handler function signature.
#[derive(Debug)]
struct HandlerParam {
    name: syn::Ident,
    ty: Box<Type>,
    kind: ParamKind,
}

#[derive(Debug, Clone, PartialEq)]
enum ParamKind {
    Body,
    Path,
    Query,
}

/// Represents a fully parsed handler method.
struct ParsedHandler {
    fn_name: syn::Ident,
    method: String,
    path: String,
    params: Vec<HandlerParam>,
    return_type: Box<Type>,
    // The original method (with &self and annotations stripped) to keep in the impl block
    clean_method: ImplItemFn,
    visibility: syn::Visibility,
}

/// Check if an attribute list contains a specific helper attribute like `#[body]`, `#[path]`, or `#[query]`.
fn take_param_attr(attrs: &[Attribute]) -> Option<ParamKind> {
    for attr in attrs {
        if attr.path().is_ident("body") {
            return Some(ParamKind::Body);
        }
        if attr.path().is_ident("path") {
            return Some(ParamKind::Path);
        }
        if attr.path().is_ident("query") {
            return Some(ParamKind::Query);
        }
    }
    None
}

/// Strip `#[body]`, `#[path]`, `#[query]`, and `#[api_handler(...)]` attributes so they don't
/// confuse the compiler in the output.
fn strip_helper_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|a| {
            !a.path().is_ident("body")
                && !a.path().is_ident("path")
                && !a.path().is_ident("query")
        })
        .cloned()
        .collect()
}

fn strip_api_handler_attr(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|a| !a.path().is_ident("api_handler"))
        .cloned()
        .collect()
}

/// Extract the inner type `T` from `MyAppResult<T>` (or any single-generic wrapper).
fn extract_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return inner;
                }
            }
        }
    }
    ty
}

/// Build the path string for axum routes.
/// Axum 0.8+ uses `{name}` syntax for path parameters, which matches our input format.
fn path_to_axum(path: &str) -> &str {
    path
}

fn parse_handler(method: &ImplItemFn) -> Option<ParsedHandler> {
    // Look for #[api_handler(...)] attribute
    let api_attr = method
        .attrs
        .iter()
        .find(|a| a.path().is_ident("api_handler"))?;

    let meta = &api_attr.meta;
    let args = ApiHandlerArgs::from_meta(&meta).expect("Failed to parse #[api_handler] arguments");

    let fn_name = method.sig.ident.clone();
    let visibility = method.vis.clone();

    // Parse parameters (skip &self)
    let mut params = Vec::new();
    for arg in method.sig.inputs.iter().skip(1) {
        if let FnArg::Typed(PatType { pat, ty, attrs, .. }) = arg {
            if let Pat::Ident(pat_ident) = pat.as_ref() {
                let kind = take_param_attr(attrs).unwrap_or_else(|| {
                    panic!(
                        "Parameter `{}` in `{}` must be annotated with #[body], #[path], or #[query]",
                        pat_ident.ident, fn_name
                    )
                });
                params.push(HandlerParam {
                    name: pat_ident.ident.clone(),
                    ty: ty.clone(),
                    kind,
                });
            }
        }
    }

    // Extract return type
    let return_type = match &method.sig.output {
        ReturnType::Type(_, ty) => ty.clone(),
        ReturnType::Default => panic!("Handler `{}` must have a return type", fn_name),
    };

    // Build a clean version of the method: strip helper attrs from params and method
    let mut clean_method = method.clone();
    clean_method.attrs = strip_api_handler_attr(&clean_method.attrs);

    // Strip #[body] / #[path] from params in the clean method
    for arg in clean_method.sig.inputs.iter_mut() {
        if let FnArg::Typed(pat_type) = arg {
            pat_type.attrs = strip_helper_attrs(&pat_type.attrs);
        }
    }

    Some(ParsedHandler {
        fn_name,
        method: args.method.to_uppercase(),
        path: args.path,
        params,
        return_type,
        clean_method,
        visibility,
    })
}

/// Generate the free-standing axum handler function for a parsed handler.
fn generate_axum_handler(struct_name: &syn::Ident, handler: &ParsedHandler) -> TokenStream2 {
    let fn_name = &handler.fn_name;
    let handler_fn_name = format_ident!("__axum_handler_{}", fn_name);

    // Build extractor arguments and call arguments
    let mut extractor_params: Vec<TokenStream2> = Vec::new();
    let mut call_args: Vec<TokenStream2> = Vec::new();

    // State is always first (before body, which must be last in axum)
    extractor_params.push(quote! {
        axum::extract::State(state): axum::extract::State<std::sync::Arc<#struct_name>>
    });

    // Path params come before query and body
    let path_params: Vec<_> = handler
        .params
        .iter()
        .filter(|p| p.kind == ParamKind::Path)
        .collect();

    if path_params.len() == 1 {
        let name = &path_params[0].name;
        let ty = &path_params[0].ty;
        extractor_params.push(quote! {
            axum::extract::Path(#name): axum::extract::Path<#ty>
        });
        call_args.push(quote! { #name });
    } else if path_params.len() > 1 {
        // Multiple path params → extract as tuple
        let names: Vec<_> = path_params.iter().map(|p| &p.name).collect();
        let types: Vec<_> = path_params.iter().map(|p| &p.ty).collect();
        extractor_params.push(quote! {
            axum::extract::Path((#(#names),*)): axum::extract::Path<(#(#types),*)>
        });
        for name in &names {
            call_args.push(quote! { #name });
        }
    }

    // Query params come before body
    for param in handler.params.iter().filter(|p| p.kind == ParamKind::Query) {
        let name = &param.name;
        let ty = &param.ty;
        extractor_params.push(quote! {
            axum::extract::Query(#name): axum::extract::Query<#ty>
        });
        call_args.push(quote! { #name });
    }

    // Body param (must be last for axum)
    for param in handler.params.iter().filter(|p| p.kind == ParamKind::Body) {
        let name = &param.name;
        let ty = &param.ty;
        extractor_params.push(quote! {
            axum::extract::Json(#name): axum::extract::Json<#ty>
        });
        call_args.push(quote! { #name });
    }

    quote! {
        async fn #handler_fn_name(
            #(#extractor_params),*
        ) -> impl axum::response::IntoResponse {
            state.#fn_name(#(#call_args),*).await
        }
    }
}

/// Generate the `fn router()` method that wires all handlers to their routes.
fn generate_router(_struct_name: &syn::Ident, handlers: &[ParsedHandler]) -> TokenStream2 {
    let mut route_calls: Vec<TokenStream2> = Vec::new();

    for handler in handlers {
        let handler_fn_name = format_ident!("__axum_handler_{}", handler.fn_name);
        let axum_path = path_to_axum(&handler.path);

        let method_fn = match handler.method.as_str() {
            "GET" => quote! { axum::routing::get },
            "POST" => quote! { axum::routing::post },
            "PUT" => quote! { axum::routing::put },
            "DELETE" => quote! { axum::routing::delete },
            "PATCH" => quote! { axum::routing::patch },
            other => panic!("Unsupported HTTP method: {}", other),
        };

        route_calls.push(quote! {
            .route(#axum_path, #method_fn(#handler_fn_name))
        });
    }

    quote! {
        /// Build an axum Router with all annotated handlers wired up.
        pub fn router(self: std::sync::Arc<Self>) -> axum::Router {
            axum::Router::new()
                #(#route_calls)*
                .with_state(self)
        }
    }
}

/// Generate the client struct and its impl block.
fn generate_client(struct_name: &syn::Ident, handlers: &[ParsedHandler]) -> TokenStream2 {
    let client_name = format_ident!("{}Client", struct_name);
    let error_name = format_ident!("{}ClientError", struct_name);

    let mut client_methods: Vec<TokenStream2> = Vec::new();

    for handler in handlers {
        let fn_name = &handler.fn_name;
        let vis = &handler.visibility;
        let inner_return_type = extract_inner_type(&handler.return_type);

        // Build function parameters (no &self yet, we add it)
        let mut fn_params: Vec<TokenStream2> = Vec::new();
        let mut path_format_args: Vec<TokenStream2> = Vec::new();
        let mut body_arg: Option<TokenStream2> = None;
        let mut query_arg: Option<TokenStream2> = None;

        for param in &handler.params {
            let name = &param.name;
            let ty = &param.ty;
            match param.kind {
                ParamKind::Path => {
                    fn_params.push(quote! { #name: #ty });
                    path_format_args.push(quote! { #name = #name });
                }
                ParamKind::Body => {
                    fn_params.push(quote! { #name: &#ty });
                    body_arg = Some(quote! { #name });
                }
                ParamKind::Query => {
                    fn_params.push(quote! { #name: &#ty });
                    query_arg = Some(quote! { #name });
                }
            }
        }

        // Build the URL expression
        let path_str = &handler.path;
        let base_url_expr = if path_format_args.is_empty() {
            quote! { format!("{}{}", self.base_url, #path_str) }
        } else {
            quote! { format!(concat!("{}", #path_str), self.base_url, #(#path_format_args),*) }
        };

        // Add query string if there's a query param
        let url_expr = if let Some(query) = &query_arg {
            quote! {
                {
                    let base = #base_url_expr;
                    let query_string = serde_urlencoded::to_string(#query)
                        .expect("failed to serialize query parameters");
                    if query_string.is_empty() {
                        base
                    } else {
                        format!("{}?{}", base, query_string)
                    }
                }
            }
        } else {
            base_url_expr
        };

        // Build the request chain
        let method_lower = handler.method.to_lowercase();
        let method_ident = format_ident!("{}", method_lower);

        let url_ident = format_ident!("url");
        let request_chain = if let Some(body) = &body_arg {
            quote! {
                self.client
                    .#method_ident(&#url_ident)
                    .json(#body)
                    .send()
                    .await
            }
        } else {
            quote! {
                self.client
                    .#method_ident(&#url_ident)
                    .send()
                    .await
            }
        };

        client_methods.push(quote! {
            #vis async fn #fn_name(&self, #(#fn_params),*) -> Result<#inner_return_type, #error_name> {
                let url = #url_expr;
                let response = #request_chain
                    .map_err(#error_name::Request)?;

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    return Err(#error_name::Api { status, body });
                }

                response
                    .json::<#inner_return_type>()
                    .await
                    .map_err(#error_name::Request)
            }
        });
    }

    quote! {
        /// Auto-generated HTTP client for the API.
        pub struct #client_name {
            base_url: String,
            client: reqwest::Client,
        }

        #[derive(Debug)]
        pub enum #error_name {
            Request(reqwest::Error),
            Api {
                status: reqwest::StatusCode,
                body: String,
            },
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Request(e) => write!(f, "HTTP request error: {e}"),
                    Self::Api { status, body } => write!(f, "API error ({status}): {body}"),
                }
            }
        }

        impl std::error::Error for #error_name {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    Self::Request(e) => Some(e),
                    Self::Api { .. } => None,
                }
            }
        }

        impl #client_name {
            /// Create a new client pointing at the given base URL (e.g. `"http://localhost:3000"`).
            pub fn new(base_url: impl Into<String>) -> Self {
                Self {
                    base_url: base_url.into(),
                    client: reqwest::Client::new(),
                }
            }

            /// Create a new client with a custom `reqwest::Client`.
            pub fn with_client(base_url: impl Into<String>, client: reqwest::Client) -> Self {
                Self {
                    base_url: base_url.into(),
                    client,
                }
            }

            #(#client_methods)*
        }
    }
}

/// The main attribute macro: `#[api]`
///
/// Place this on an `impl` block. Methods annotated with `#[api_handler(...)]`
/// will have axum handler functions and a typed HTTP client generated automatically.
#[proc_macro_attribute]
pub fn api(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemImpl);

    // Get the struct name
    let struct_name = if let Type::Path(type_path) = input.self_ty.as_ref() {
        type_path
            .path
            .segments
            .last()
            .expect("Expected a struct name")
            .ident
            .clone()
    } else {
        panic!("#[api] must be applied to an impl block for a named struct");
    };

    // Parse all handler methods
    let mut handlers: Vec<ParsedHandler> = Vec::new();
    let mut cleaned_items: Vec<ImplItem> = Vec::new();

    for item in &input.items {
        if let ImplItem::Fn(method) = item {
            if let Some(parsed) = parse_handler(method) {
                cleaned_items.push(ImplItem::Fn(parsed.clean_method.clone()));
                handlers.push(parsed);
            } else {
                // Not an api_handler method — keep as-is
                cleaned_items.push(item.clone());
            }
        } else {
            cleaned_items.push(item.clone());
        }
    }

    input.items = cleaned_items;

    // Generate axum handler functions
    let axum_handlers: Vec<TokenStream2> = handlers
        .iter()
        .map(|h| generate_axum_handler(&struct_name, h))
        .collect();

    // Generate router method
    let router_impl = generate_router(&struct_name, &handlers);

    // Generate client
    let client = generate_client(&struct_name, &handlers);

    let expanded = quote! {
        #input

        impl #struct_name {
            #router_impl
        }

        // Free-standing handler functions (module-private)
        #(#axum_handlers)*

        #client
    };

    TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    // ── Tests for take_param_attr() ─────────────────────────────────────────

    #[test]
    fn take_param_attr_body() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[body])];
        assert_eq!(take_param_attr(&attrs), Some(ParamKind::Body));
    }

    #[test]
    fn take_param_attr_path() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[path])];
        assert_eq!(take_param_attr(&attrs), Some(ParamKind::Path));
    }

    #[test]
    fn take_param_attr_unrelated() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[serde(rename = "foo")])];
        assert_eq!(take_param_attr(&attrs), None);
    }

    #[test]
    fn take_param_attr_body_with_others() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[doc = "some doc"]),
            parse_quote!(#[body]),
            parse_quote!(#[allow(unused)]),
        ];
        assert_eq!(take_param_attr(&attrs), Some(ParamKind::Body));
    }

    #[test]
    fn take_param_attr_path_with_others() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[doc = "some doc"]), parse_quote!(#[path])];
        assert_eq!(take_param_attr(&attrs), Some(ParamKind::Path));
    }

    #[test]
    fn take_param_attr_empty() {
        let attrs: Vec<Attribute> = vec![];
        assert_eq!(take_param_attr(&attrs), None);
    }

    #[test]
    fn take_param_attr_query() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[query])];
        assert_eq!(take_param_attr(&attrs), Some(ParamKind::Query));
    }

    #[test]
    fn take_param_attr_query_with_others() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[doc = "some doc"]), parse_quote!(#[query])];
        assert_eq!(take_param_attr(&attrs), Some(ParamKind::Query));
    }

    // ── Tests for strip_helper_attrs() ──────────────────────────────────────

    #[test]
    fn strip_helper_attrs_removes_body() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[body]), parse_quote!(#[doc = "kept"])];
        let stripped = strip_helper_attrs(&attrs);
        assert_eq!(stripped.len(), 1);
        assert!(stripped[0].path().is_ident("doc"));
    }

    #[test]
    fn strip_helper_attrs_removes_path() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[path]), parse_quote!(#[allow(unused)])];
        let stripped = strip_helper_attrs(&attrs);
        assert_eq!(stripped.len(), 1);
        assert!(stripped[0].path().is_ident("allow"));
    }

    #[test]
    fn strip_helper_attrs_removes_both() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[body]),
            parse_quote!(#[path]),
            parse_quote!(#[doc = "kept"]),
        ];
        let stripped = strip_helper_attrs(&attrs);
        assert_eq!(stripped.len(), 1);
        assert!(stripped[0].path().is_ident("doc"));
    }

    #[test]
    fn strip_helper_attrs_removes_query() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[query]), parse_quote!(#[doc = "kept"])];
        let stripped = strip_helper_attrs(&attrs);
        assert_eq!(stripped.len(), 1);
        assert!(stripped[0].path().is_ident("doc"));
    }

    #[test]
    fn strip_helper_attrs_removes_all_three() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[body]),
            parse_quote!(#[path]),
            parse_quote!(#[query]),
            parse_quote!(#[doc = "kept"]),
        ];
        let stripped = strip_helper_attrs(&attrs);
        assert_eq!(stripped.len(), 1);
        assert!(stripped[0].path().is_ident("doc"));
    }

    #[test]
    fn strip_helper_attrs_keeps_unrelated() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[serde(rename = "foo")]),
            parse_quote!(#[allow(unused)]),
        ];
        let stripped = strip_helper_attrs(&attrs);
        assert_eq!(stripped.len(), 2);
    }

    #[test]
    fn strip_helper_attrs_empty() {
        let attrs: Vec<Attribute> = vec![];
        let stripped = strip_helper_attrs(&attrs);
        assert!(stripped.is_empty());
    }

    // ── Tests for strip_api_handler_attr() ──────────────────────────────────

    #[test]
    fn strip_api_handler_attr_removes_it() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[api_handler(method = "GET", path = "/foo")]),
            parse_quote!(#[doc = "kept"]),
        ];
        let stripped = strip_api_handler_attr(&attrs);
        assert_eq!(stripped.len(), 1);
        assert!(stripped[0].path().is_ident("doc"));
    }

    #[test]
    fn strip_api_handler_attr_keeps_others() {
        let attrs: Vec<Attribute> = vec![
            parse_quote!(#[doc = "some doc"]),
            parse_quote!(#[allow(unused)]),
        ];
        let stripped = strip_api_handler_attr(&attrs);
        assert_eq!(stripped.len(), 2);
    }

    // ── Tests for extract_inner_type() ──────────────────────────────────────

    #[test]
    fn extract_inner_type_result() {
        let ty: Type = parse_quote!(MyAppResult<User>);
        let inner = extract_inner_type(&ty);
        let expected: Type = parse_quote!(User);
        assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());
    }

    #[test]
    fn extract_inner_type_option() {
        let ty: Type = parse_quote!(Option<String>);
        let inner = extract_inner_type(&ty);
        let expected: Type = parse_quote!(String);
        assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());
    }

    #[test]
    fn extract_inner_type_nested() {
        let ty: Type = parse_quote!(Result<Option<User>, Error>);
        let inner = extract_inner_type(&ty);
        // Should extract the first generic arg, which is Option<User>
        let expected: Type = parse_quote!(Option<User>);
        assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());
    }

    #[test]
    fn extract_inner_type_no_generic() {
        let ty: Type = parse_quote!(String);
        let inner = extract_inner_type(&ty);
        let expected: Type = parse_quote!(String);
        assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());
    }

    #[test]
    fn extract_inner_type_unit() {
        let ty: Type = parse_quote!(MyAppResult<()>);
        let inner = extract_inner_type(&ty);
        let expected: Type = parse_quote!(());
        assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());
    }

    // ── Tests for path_to_axum() ────────────────────────────────────────────

    #[test]
    fn path_to_axum_simple() {
        assert_eq!(path_to_axum("/users"), "/users");
    }

    #[test]
    fn path_to_axum_with_param() {
        assert_eq!(path_to_axum("/users/{id}"), "/users/{id}");
    }

    #[test]
    fn path_to_axum_with_multiple_params() {
        assert_eq!(
            path_to_axum("/users/{user_id}/posts/{post_id}"),
            "/users/{user_id}/posts/{post_id}"
        );
    }

    // ── Tests for parse_handler() ───────────────────────────────────────────

    #[test]
    fn parse_handler_post_with_body() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "POST", path = "/users")]
            pub async fn create_user(&self, #[body] req: CreateUserRequest) -> MyAppResult<CreateUserResponse> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.fn_name.to_string(), "create_user");
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.path, "/users");
        assert_eq!(parsed.params.len(), 1);
        assert_eq!(parsed.params[0].name.to_string(), "req");
        assert_eq!(parsed.params[0].kind, ParamKind::Body);
    }

    #[test]
    fn parse_handler_get_with_path() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users/{id}")]
            pub async fn get_user(&self, #[path] id: UserId) -> MyAppResult<GetUserResponse> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.fn_name.to_string(), "get_user");
        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.path, "/users/{id}");
        assert_eq!(parsed.params.len(), 1);
        assert_eq!(parsed.params[0].name.to_string(), "id");
        assert_eq!(parsed.params[0].kind, ParamKind::Path);
    }

    #[test]
    fn parse_handler_put_with_path_and_body() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "PUT", path = "/users/{id}")]
            pub async fn update_user(&self, #[path] id: UserId, #[body] req: UpdateUserRequest) -> MyAppResult<UpdateUserResponse> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.fn_name.to_string(), "update_user");
        assert_eq!(parsed.method, "PUT");
        assert_eq!(parsed.path, "/users/{id}");
        assert_eq!(parsed.params.len(), 2);
        assert_eq!(parsed.params[0].name.to_string(), "id");
        assert_eq!(parsed.params[0].kind, ParamKind::Path);
        assert_eq!(parsed.params[1].name.to_string(), "req");
        assert_eq!(parsed.params[1].kind, ParamKind::Body);
    }

    #[test]
    fn parse_handler_delete() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "DELETE", path = "/users/{id}")]
            pub async fn delete_user(&self, #[path] id: UserId) -> MyAppResult<()> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.fn_name.to_string(), "delete_user");
        assert_eq!(parsed.method, "DELETE");
    }

    #[test]
    fn parse_handler_patch() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "PATCH", path = "/users/{id}")]
            pub async fn patch_user(&self, #[path] id: UserId, #[body] req: PatchRequest) -> MyAppResult<PatchResponse> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.method, "PATCH");
    }

    #[test]
    fn parse_handler_lowercase_method() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "get", path = "/health")]
            pub async fn health(&self) -> MyAppResult<String> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.method, "GET"); // Should be uppercased
    }

    #[test]
    fn parse_handler_no_api_handler_attr_returns_none() {
        let method: ImplItemFn = parse_quote! {
            pub fn regular_method(&self) -> String {
                "hello".to_string()
            }
        };

        assert!(parse_handler(&method).is_none());
    }

    #[test]
    fn parse_handler_no_params() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/health")]
            pub async fn health(&self) -> MyAppResult<String> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert!(parsed.params.is_empty());
    }

    #[test]
    fn parse_handler_get_with_query() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users")]
            pub async fn list_users(&self, #[query] params: ListUsersParams) -> MyAppResult<Vec<User>> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.fn_name.to_string(), "list_users");
        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.path, "/users");
        assert_eq!(parsed.params.len(), 1);
        assert_eq!(parsed.params[0].name.to_string(), "params");
        assert_eq!(parsed.params[0].kind, ParamKind::Query);
    }

    #[test]
    fn parse_handler_get_with_path_and_query() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users/{id}/posts")]
            pub async fn list_user_posts(&self, #[path] id: UserId, #[query] params: Pagination) -> MyAppResult<Vec<Post>> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        assert_eq!(parsed.fn_name.to_string(), "list_user_posts");
        assert_eq!(parsed.params.len(), 2);
        assert_eq!(parsed.params[0].name.to_string(), "id");
        assert_eq!(parsed.params[0].kind, ParamKind::Path);
        assert_eq!(parsed.params[1].name.to_string(), "params");
        assert_eq!(parsed.params[1].kind, ParamKind::Query);
    }

    #[test]
    fn parse_handler_preserves_visibility() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/internal")]
            pub(crate) async fn internal_endpoint(&self) -> MyAppResult<String> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");
        // Check visibility is preserved (pub(crate))
        let vis_str = quote!(#(parsed.visibility)).to_string();
        assert!(
            vis_str.contains("crate")
                || matches!(parsed.visibility, syn::Visibility::Restricted(_))
        );
    }

    #[test]
    fn parse_handler_strips_attrs_in_clean_method() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "POST", path = "/users")]
            pub async fn create_user(&self, #[body] req: CreateUserRequest) -> MyAppResult<CreateUserResponse> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).expect("Should parse successfully");

        // Check that api_handler attr is stripped from clean_method
        let has_api_handler = parsed
            .clean_method
            .attrs
            .iter()
            .any(|a| a.path().is_ident("api_handler"));
        assert!(!has_api_handler, "api_handler should be stripped");

        // Check that #[body] is stripped from params in clean_method
        for arg in parsed.clean_method.sig.inputs.iter().skip(1) {
            if let FnArg::Typed(pat_type) = arg {
                let has_body = pat_type.attrs.iter().any(|a| a.path().is_ident("body"));
                assert!(!has_body, "#[body] should be stripped from clean_method");
            }
        }
    }

    // ── Tests for generate_axum_handler() ───────────────────────────────────

    #[test]
    fn generate_axum_handler_includes_state() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/health")]
            pub async fn health(&self) -> MyAppResult<String> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).unwrap();
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_axum_handler(&struct_name, &parsed);
        let code = generated.to_string();

        assert!(code.contains("State"));
        assert!(code.contains("MyApp"));
        assert!(code.contains("__axum_handler_health"));
    }

    #[test]
    fn generate_axum_handler_with_body() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "POST", path = "/users")]
            pub async fn create_user(&self, #[body] req: CreateUserRequest) -> MyAppResult<CreateUserResponse> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).unwrap();
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_axum_handler(&struct_name, &parsed);
        let code = generated.to_string();

        assert!(code.contains("Json"));
        assert!(code.contains("CreateUserRequest"));
    }

    #[test]
    fn generate_axum_handler_with_path_param() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users/{id}")]
            pub async fn get_user(&self, #[path] id: UserId) -> MyAppResult<GetUserResponse> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).unwrap();
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_axum_handler(&struct_name, &parsed);
        let code = generated.to_string();

        assert!(code.contains("Path"));
        assert!(code.contains("UserId"));
    }

    #[test]
    fn generate_axum_handler_with_query_param() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users")]
            pub async fn list_users(&self, #[query] params: ListUsersParams) -> MyAppResult<Vec<User>> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).unwrap();
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_axum_handler(&struct_name, &parsed);
        let code = generated.to_string();

        assert!(code.contains("Query"));
        assert!(code.contains("ListUsersParams"));
    }

    #[test]
    fn generate_axum_handler_with_path_and_query() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users/{id}/posts")]
            pub async fn list_user_posts(&self, #[path] id: UserId, #[query] params: Pagination) -> MyAppResult<Vec<Post>> {
                todo!()
            }
        };

        let parsed = parse_handler(&method).unwrap();
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_axum_handler(&struct_name, &parsed);
        let code = generated.to_string();

        assert!(code.contains("Path"));
        assert!(code.contains("UserId"));
        assert!(code.contains("Query"));
        assert!(code.contains("Pagination"));
    }

    // ── Tests for generate_router() ─────────────────────────────────────────

    #[test]
    fn generate_router_creates_routes() {
        let method1: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users")]
            pub async fn list_users(&self) -> MyAppResult<Vec<User>> {
                todo!()
            }
        };
        let method2: ImplItemFn = parse_quote! {
            #[api_handler(method = "POST", path = "/users")]
            pub async fn create_user(&self, #[body] req: CreateUserRequest) -> MyAppResult<User> {
                todo!()
            }
        };

        let handlers = vec![
            parse_handler(&method1).unwrap(),
            parse_handler(&method2).unwrap(),
        ];
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_router(&struct_name, &handlers);
        let code = generated.to_string();

        assert!(code.contains("Router"));
        assert!(code.contains("route"));
        assert!(code.contains("\"/users\""));
        assert!(code.contains("get"));
        assert!(code.contains("post"));
    }

    // ── Tests for generate_client() ─────────────────────────────────────────

    #[test]
    fn generate_client_creates_struct() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/health")]
            pub async fn health(&self) -> MyAppResult<String> {
                todo!()
            }
        };

        let handlers = vec![parse_handler(&method).unwrap()];
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_client(&struct_name, &handlers);
        let code = generated.to_string();

        assert!(code.contains("MyAppClient"));
        assert!(code.contains("MyAppClientError"));
        assert!(code.contains("base_url"));
        assert!(code.contains("reqwest :: Client"));
    }

    #[test]
    fn generate_client_creates_error_type() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/health")]
            pub async fn health(&self) -> MyAppResult<String> {
                todo!()
            }
        };

        let handlers = vec![parse_handler(&method).unwrap()];
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_client(&struct_name, &handlers);
        let code = generated.to_string();

        assert!(code.contains("Request"));
        assert!(code.contains("Api"));
        assert!(code.contains("status"));
        assert!(code.contains("body"));
    }

    #[test]
    fn generate_client_method_with_body_takes_reference() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "POST", path = "/users")]
            pub async fn create_user(&self, #[body] req: CreateUserRequest) -> MyAppResult<User> {
                todo!()
            }
        };

        let handlers = vec![parse_handler(&method).unwrap()];
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_client(&struct_name, &handlers);
        let code = generated.to_string();

        // Body param should be taken by reference
        assert!(code.contains("req : & CreateUserRequest"));
    }

    #[test]
    fn generate_client_method_with_path_takes_value() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users/{id}")]
            pub async fn get_user(&self, #[path] id: UserId) -> MyAppResult<User> {
                todo!()
            }
        };

        let handlers = vec![parse_handler(&method).unwrap()];
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_client(&struct_name, &handlers);
        let code = generated.to_string();

        // Path param should be taken by value (no &)
        assert!(code.contains("id : UserId"));
        assert!(!code.contains("id : & UserId"));
    }

    #[test]
    fn generate_client_method_with_query_takes_reference() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users")]
            pub async fn list_users(&self, #[query] params: ListUsersParams) -> MyAppResult<Vec<User>> {
                todo!()
            }
        };

        let handlers = vec![parse_handler(&method).unwrap()];
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_client(&struct_name, &handlers);
        let code = generated.to_string();

        // Query param should be taken by reference
        assert!(code.contains("params : & ListUsersParams"));
        // Should use serde_urlencoded
        assert!(code.contains("serde_urlencoded"));
    }

    #[test]
    fn generate_client_method_with_path_and_query() {
        let method: ImplItemFn = parse_quote! {
            #[api_handler(method = "GET", path = "/users/{id}/posts")]
            pub async fn list_user_posts(&self, #[path] id: UserId, #[query] params: Pagination) -> MyAppResult<Vec<Post>> {
                todo!()
            }
        };

        let handlers = vec![parse_handler(&method).unwrap()];
        let struct_name: syn::Ident = parse_quote!(MyApp);
        let generated = generate_client(&struct_name, &handlers);
        let code = generated.to_string();

        // Path param should be taken by value
        assert!(code.contains("id : UserId"));
        // Query param should be taken by reference
        assert!(code.contains("params : & Pagination"));
        // Should use serde_urlencoded
        assert!(code.contains("serde_urlencoded"));
    }
}
