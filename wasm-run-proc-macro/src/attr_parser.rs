use syn::parse::{Error, ParseStream, Result};
use syn::{Ident, LitStr, Path, Token};

pub struct Attr {
    pub other_cli_commands: Option<Path>,
    #[cfg(not(feature = "serve"))]
    pub run_server: Option<Path>,
    pub prepare_build: Option<Path>,
    pub post_build: Option<Path>,
    #[cfg(feature = "serve")]
    pub serve: Option<Path>,
    pub watch: Option<Path>,
    pub crate_name: Option<LitStr>,
}

impl Attr {
    pub fn parse(input: ParseStream) -> Result<Self> {
        let crate_name = input.parse().ok();

        if crate_name.is_some() {
            input.parse::<Token![,]>()?;
        }

        let mut other_cli_commands = None;
        #[cfg(not(feature = "serve"))]
        let mut run_server = None;
        let mut prepare_build = None;
        let mut post_build = None;
        #[cfg(feature = "serve")]
        let mut serve = None;
        let mut watch = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let _eq_token: Token![=] = input.parse()?;
            let path: Path = input.parse()?;

            match ident.to_string().as_str() {
                "other_cli_commands" => other_cli_commands = Some(path),
                "prepare_build" => prepare_build = Some(path),
                "post_build" => post_build = Some(path),
                #[cfg(feature = "serve")]
                "serve" => serve = Some(path),
                #[cfg(not(feature = "serve"))]
                "run_server" => run_server = Some(path),
                "watch" => watch = Some(path),
                _ => return Err(Error::new(ident.span(), "invalid argument")),
            }

            let _comma_token: Token![,] = match input.parse() {
                Ok(x) => x,
                Err(_) if input.is_empty() => break,
                Err(err) => return Err(err),
            };
        }

        Ok(Self {
            other_cli_commands,
            #[cfg(not(feature = "serve"))]
            run_server,
            prepare_build,
            post_build,
            #[cfg(feature = "serve")]
            serve,
            watch,
            crate_name,
        })
    }
}
