use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Ident, ItemEnum, Path};

pub fn generate<'a>(
    item: ItemEnum,
    other_cli_commands: Option<Path>,
    hooks: impl Iterator<Item = (&'a Ident, &'a Path)>,
    build: Option<Path>,
    serve: Option<Path>,
) -> syn::Result<TokenStream> {
    let build = build.unwrap_or_else(|| syn::parse_str("::wasm_run::DefaultBuildArgs").unwrap());
    let serve = serve.unwrap_or_else(|| syn::parse_str("::wasm_run::DefaultServeArgs").unwrap());

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

    let other_cli_commands = other_cli_commands
        .map(|x| quote! { #x(cli)? })
        .unwrap_or_else(|| quote! { {} });

    Ok(quote! {
        #( #attrs )*
        #vis enum #ident #generics {
            Serve(#serve),
            Build(#build),
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
