use crate::attr_parser::Attr;
use cargo_metadata::Metadata;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Fields, ItemEnum};

pub fn generate(item: ItemEnum, attr: Attr, metadata: &Metadata) -> syn::Result<TokenStream> {
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
        pre_build,
        post_build,
        #[cfg(feature = "serve")]
        serve,
        watch,
        pkg_name,
        default_build_path,
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

    let span = other_cli_commands.span();
    let other_cli_commands = other_cli_commands
        .map(|x| quote_spanned! {span=> cli => #x(cli, metadata, package)?, })
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

    let pre_build = pre_build.map(|path| {
        quote_spanned! {path.span()=>
            pre_build: Box::new(|args, profile, command| {
                let args = args.downcast_ref::<#build_ty>()
                    .expect("invalid type for `Build` command: the type in the command enum \
                        must be the same than the type returned by `build_args()` \
                        in the implementation of the trait `ServeArgs`");
                #path(args, profile, command)
            }),
        }
    });

    let post_build = post_build.map(|path| {
        quote_spanned! {path.span()=>
            post_build: Box::new(|args, profile, wasm_js, wasm_bin| {
                let args = args.downcast_ref::<#build_ty>()
                    .expect("invalid type for `Build` command: the type in the command enum \
                        must be the same than the type returned by `build_args()` \
                        in the implementation of the trait `ServeArgs`");
                #path(args, profile, wasm_js, wasm_bin)
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

    let mut check_package_existence = quote! {};
    if let Some(pkg_name) = pkg_name.as_ref() {
        let span = pkg_name.span();
        let pkg_name = pkg_name.value();
        if metadata
            .packages
            .iter()
            .find(|x| x.name == pkg_name)
            .is_none()
        {
            let message = format!("package `{}` not found", pkg_name);
            check_package_existence = quote_spanned! {span=> compile_error!(#message); };
        }
    }

    let pkg_name = pkg_name.map(|x| quote! { #x }).unwrap_or_else(|| {
        let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
        quote! { #pkg_name }
    });

    #[cfg(feature = "serve")]
    let run_server_arm = quote! {};
    #[cfg(not(feature = "serve"))]
    let run_server_arm = if let Some(run_server) = run_server {
        quote_spanned! {run_server.span()=>
            #ident::RunServer(args) => #run_server(args)?,
        }
    } else {
        quote! {
            _ => compile_error!(
                "without the feature `serve` you need to provide a `run_server` argument to the \
                macro. Example: #[main(run_server = my_awesome_function)]",
            ),
        }
    };

    let default_build_path = if let Some(path) = default_build_path {
        quote_spanned! {path.span()=>
            Some(Box::new(|metadata, package| {
                #path(metadata, package)
            }))
        }
    } else {
        quote! { None }
    };

    Ok(quote! {
        #check_package_existence

        #( #attrs )*
        #vis enum #ident #generics {
            #serve_variant
            #build_variant
            #run_variant
            #variants
        }

        fn main() -> ::wasm_run::prelude::anyhow::Result<()> {
            use ::std::path::PathBuf;
            use ::wasm_run::structopt::StructOpt;
            use ::wasm_run::prelude::*;

            let cli = #ident::from_args();

            let (metadata, package) = ::wasm_run::wasm_run_init(#pkg_name, #default_build_path)?;

            #[allow(clippy::needless_update)]
            let hooks = ::wasm_run::Hooks {
                #pre_build
                #post_build
                #serve
                #watch
                .. Hooks::default()
            };

            match cli {
                #ident::Build(args) => args.run(hooks)?,
                #ident::Serve(args) => args.run(hooks)?,
                #run_server_arm
                #other_cli_commands
            }

            Ok(())
        }
    })
}
