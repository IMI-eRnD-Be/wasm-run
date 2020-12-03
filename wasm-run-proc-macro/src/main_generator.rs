use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Ident, ItemEnum, Path};

pub fn generate<'a>(
    item: ItemEnum,
    other_cli_commands: Option<Path>,
    hooks: impl Iterator<Item = (&'a Ident, &'a Path)>,
) -> syn::Result<TokenStream> {
    let hooks = hooks
        .map(|(ident, path)| quote_spanned! {ident.span()=> #ident: Box::new(#path), })
        .collect::<Vec<_>>();

    let ItemEnum {
        attrs,
        vis,
        ident,
        generics,
        variants,
        ..
    } = item;

    let build = if variants.iter().find(|x| x.ident == "Build").is_none() {
        Some(quote! { Build(::wasm_run::DefaultBuildArgs), })
    } else {
        None
    };

    let serve = if variants.iter().find(|x| x.ident == "Serve").is_none() {
        Some(quote! { Serve(::wasm_run::DefaultServeArgs), })
    } else {
        None
    };

    let other_cli_commands = other_cli_commands
        .map(|x| quote! { #x(cli)? })
        .unwrap_or_else(|| quote! { {} });

    Ok(quote! {
        #( #attrs )*
        #vis enum #ident #generics {
            #serve
            #build
            #variants
        }

        fn main() -> ::wasm_run::anyhow::Result<()> {
            use ::wasm_run::structopt::StructOpt;
            use ::wasm_run::{BuildArgs, ServeArgs};

            let cli = #ident::from_args();
            let hooks = ::wasm_run::Hooks {
                #( #hooks )*
                .. ::wasm_run::Hooks::default()
            };

            match cli {
                #ident::Build(args) => args.run(hooks)?,
                #ident::Serve(args) => args.run(hooks)?,
                cli => #other_cli_commands,
            }

            Ok(())
        }
    })
}
