use crate::attr_parser::Attr;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Fields, ItemEnum};

pub fn generate(item: ItemEnum, attr: Attr) -> syn::Result<TokenStream> {
    let ItemEnum {
        attrs,
        vis,
        ident,
        generics,
        variants,
        ..
    } = item;
    let Attr {
        other_cli_commands,
        #[cfg(not(feature = "serve"))]
        run_server,
        prepare_build,
        post_build,
        #[cfg(feature = "serve")]
        serve,
        watch,
        crate_name,
    } = attr;

    let (build_variant, build_ty) =
        if let Some(variant) = variants.iter().find(|x| x.ident == "Build") {
            match &variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    (None, fields.unnamed[0].ty.to_token_stream())
                }
                _ => (
                    None,
                    quote_spanned!(variant.fields.span()=>
                        compile_error!("only the tuple variant with only one struct is allowed. \
                            Example: Build(YourBuildArgs)")),
                ),
            }
        } else {
            let ty = quote!(::wasm_run::DefaultBuildArgs);
            (
                Some(quote! {
                    /// Build for production.
                    Build(#ty),
                }),
                ty,
            )
        };

    let (serve_variant, serve_ty) =
        if let Some(variant) = variants.iter().find(|x| x.ident == "Serve") {
            match &variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    (None, fields.unnamed[0].ty.to_token_stream())
                }
                _ => (
                    None,
                    quote_spanned!(variant.fields.span()=>
                        compile_error!("only the tuple variant with only one struct is allowed. \
                            Example: Serve(YourServeArgs)")),
                ),
            }
        } else {
            let ty = quote!(::wasm_run::DefaultServeArgs);
            (
                Some(quote! {
                    /// Run development server.
                    Serve(#ty),
                }),
                ty,
            )
        };

    #[cfg(feature = "serve")]
    let run_variant = quote! {};
    #[cfg(not(feature = "serve"))]
    let run_variant = quote! {
        #[structopt(setting = ::wasm_run::structopt::clap::AppSettings::Hidden)]
        RunServer(#serve_ty),
    };

    let other_cli_commands = other_cli_commands
        .map(|x| quote! { cli => #x(cli)?, })
        .unwrap_or_else(|| {
            if variants
                .iter()
                .filter(|x| x.ident != "Build" && x.ident != "Serve")
                .count()
                > 0
            {
                quote! {
                    cli => compile_error!(
                        "missing `other_cli_commands` to handle all the variants",
                    ),
                }
            } else {
                quote! {}
            }
        });

    let prepare_build = prepare_build.map(|path| {
        quote_spanned! {path.span()=>
            prepare_build: Box::new(|args, profile, wasm_js, wasm_bin| {
                let args = args.downcast_ref::<#build_ty>()
                    .expect("invalid type for `Build` command: the type in the command enum \
                        must be the same than the type returned by `build_args()` \
                        on the implementation of the trait `BuildArgs`");
                #path(args, profile, wasm_js, wasm_bin)
            }),
        }
    });

    let post_build = post_build.map(|path| {
        quote_spanned! {path.span()=>
            post_build: Box::new(|args, profile| {
                let args = args.downcast_ref::<#build_ty>().unwrap();
                #path(args, profile)
            }),
        }
    });

    #[cfg(feature = "serve")]
    let serve = serve.map(|path| {
        quote_spanned! {path.span()=>
            serve: Box::new(|args, app| {
                let args = args.downcast_ref::<#serve_ty>().unwrap();
                #path(args, app)
            }),
        }
    });
    #[cfg(not(feature = "serve"))]
    let serve = quote! {};

    let watch = watch.map(|path| {
        quote_spanned! {path.span()=>
            watch: Box::new(|args, watcher| {
                let args = args.downcast_ref::<#serve_ty>().unwrap();
                #path(args, watcher)
            }),
        }
    });

    let crate_name = crate_name.map(|x| quote! { #x }).unwrap_or_else(|| {
        let crate_name = std::env::var("CARGO_CRATE_NAME").unwrap();
        quote! { #crate_name }
    });

    #[cfg(feature = "serve")]
    let run_server_arm = quote! {};
    #[cfg(not(feature = "serve"))]
    let run_server_arm = quote! {
        #ident::RunServer(args) => #run_server(args)?,
    };

    Ok(quote! {
        #( #attrs )*
        #vis enum #ident #generics {
            #serve_variant
            #build_variant
            #run_variant
            #variants
        }

        fn main() -> ::wasm_run::anyhow::Result<()> {
            use ::wasm_run::structopt::StructOpt;
            use ::wasm_run::*;

            let cli = #ident::from_args();
            #[allow(clippy::needless_update)]
            let hooks = ::wasm_run::Hooks {
                #prepare_build
                #post_build
                #serve
                #watch
                .. ::wasm_run::Hooks::default()
            };

            let crate_name = #crate_name;
            let crate_name = #crate_name.to_string();

            match cli {
                #ident::Build(args) => args.run(crate_name, hooks)?,
                #ident::Serve(args) => args.run(crate_name, hooks)?,
                #run_server_arm
                #other_cli_commands
            }

            Ok(())
        }
    })
}
