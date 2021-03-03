use crate::attr_parser::Attr;
use cargo_metadata::Metadata;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Error, ItemEnum};

pub fn generate(item: ItemEnum, attr: Attr, metadata: &Metadata) -> syn::Result<TokenStream> {
    let ident = &item.ident;
    let Attr {
        other_cli_commands,
        pre_build,
        post_build,
        #[cfg(feature = "mini-http-server")]
        serve,
        watch,
        pkg_name,
        backend_pkg_name,
        default_build_path,
        build_args,
        serve_args,
    } = attr;

    if let Some(serve_args) = serve_args.as_ref() {
        if build_args.is_none() {
            return Err(Error::new(
                serve_args.span(),
                "if you use a custom ServeArgs, you must use a custom BuildArgs",
            ));
        }
    }

    let build_ty = if let Some(ty) = build_args {
        quote! { #ty }
    } else {
        quote! { ::wasm_run::DefaultBuildArgs }
    };

    let serve_ty = if let Some(ty) = serve_args {
        quote! { #ty }
    } else {
        quote! { ::wasm_run::DefaultServeArgs }
    };

    let span = other_cli_commands.span();
    let other_cli_commands = other_cli_commands
        .map(|x| {
            quote_spanned! {span=>
                WasmRunCliCommand::Other(cli) => #x(cli, metadata, package)?,
            }
        })
        .unwrap_or_else(|| {
            if !item.variants.is_empty() {
                quote! {
                    cli => compile_error!(
                        "missing `other_cli_commands` to handle all the variants",
                    ),
                }
            } else {
                quote! {
                    WasmRunCliCommand::Other(x) => match x {},
                }
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

    #[cfg(feature = "mini-http-server")]
    let serve = serve.map(|path| {
        quote_spanned! {path.span()=>
            serve: Box::new(|args, app| {
                let args = args.downcast_ref::<#serve_ty>().unwrap();
                #path(args, app)
            }),
        }
    });
    #[cfg(not(feature = "mini-http-server"))]
    let serve = quote! {};

    let watch = watch.map(|path| {
        quote_spanned! {path.span()=>
            watch: Box::new(|args, watcher| {
                let args = args.downcast_ref::<#serve_ty>().unwrap();
                #path(args, watcher)
            }),
        }
    });

    if let Some(pkg_name) = pkg_name.as_ref() {
        let span = pkg_name.span();
        let pkg_name = pkg_name.value();
        if metadata
            .packages
            .iter()
            .find(|x| x.name == pkg_name)
            .is_none()
        {
            return Err(Error::new(
                span,
                format!("package `{}` not found", pkg_name),
            ));
        }
    }

    let pkg_name = pkg_name.map(|x| quote! { #x }).unwrap_or_else(|| {
        let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();
        quote! { #pkg_name }
    });

    if let Some(pkg_name) = backend_pkg_name.as_ref() {
        let span = pkg_name.span();
        let pkg_name = pkg_name.value();
        if metadata
            .packages
            .iter()
            .find(|x| x.name == pkg_name)
            .is_none()
        {
            return Err(Error::new(
                span,
                format!("package `{}` not found", pkg_name),
            ));
        }
    }

    let backend_pkg_name = backend_pkg_name
        .map(|x| quote! { Some(#x) })
        .unwrap_or_else(|| {
            quote! { None }
        });

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
        #item

        impl #ident {
            fn build() -> ::wasm_run::prelude::anyhow::Result<::std::path::PathBuf>
            {
                use ::wasm_run::BuildArgs;
                let build_args = #build_ty::from_iter_safe(&[#pkg_name])?;
                build_args.run()
            }

            fn build_with_args<I>(iter: I)
            -> ::wasm_run::prelude::anyhow::Result<::std::path::PathBuf>
            where
                I: ::std::iter::IntoIterator,
                I::Item: ::std::convert::Into<::std::ffi::OsString> + Clone,
            {
                use ::wasm_run::BuildArgs;
                let iter = ::std::iter::once(::std::ffi::OsString::from(#pkg_name))
                    .chain(iter.into_iter().map(|x| x.into()));
                let build_args = #build_ty::from_iter_safe(iter)?;
                build_args.run()
            }
        }

        fn main() -> ::wasm_run::prelude::anyhow::Result<()> {
            use ::std::path::PathBuf;
            use ::wasm_run::structopt::StructOpt;
            use ::wasm_run::prelude::*;

            #[derive(::wasm_run::structopt::StructOpt)]
            struct WasmRunCli {
                #[structopt(subcommand)]
                command: Option<WasmRunCliCommand>,
            }

            #[derive(::wasm_run::structopt::StructOpt)]
            enum WasmRunCliCommand {
                Build(#build_ty),
                Serve(#serve_ty),
                #[structopt(flatten)]
                Other(#ident),
            }

            let cli = WasmRunCli::from_args();

            #[allow(clippy::needless_update)]
            let hooks = Hooks {
                #pre_build
                #post_build
                #serve
                #watch
                .. Hooks::default()
            };

            let (metadata, package) = ::wasm_run::wasm_run_init(
                #pkg_name,
                #backend_pkg_name,
                #default_build_path,
                hooks,
            )?;

            if let Some(cli) = cli.command {
                match cli {
                    WasmRunCliCommand::Build(args) => {
                        args.run()?;
                    },
                    WasmRunCliCommand::Serve(args) => args.run()?,
                    #other_cli_commands
                }
            } else {
                #serve_ty::from_args().run()?;
            }

            Ok(())
        }
    })
}
