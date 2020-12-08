use syn::parse::{Error, ParseStream, Result};
use syn::{Ident, LitStr, Path, Token};

pub struct Attr {
    pub other_cli_commands: Option<Path>,
    #[cfg(not(feature = "serve"))]
    pub run_server: Option<Path>,
    pub pre_build: Option<Path>,
    pub post_build: Option<Path>,
    #[cfg(feature = "serve")]
    pub serve: Option<Path>,
    pub watch: Option<Path>,
    pub pkg_name: Option<LitStr>,
    pub default_build_path: Option<Path>,
}

impl Attr {
    pub fn parse(input: ParseStream) -> Result<Self> {
        let pkg_name = input.parse().ok();

        if pkg_name.is_some() && !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        let mut other_cli_commands = None;
        #[cfg(not(feature = "serve"))]
        let mut run_server = None;
        let mut pre_build = None;
        let mut post_build = None;
        #[cfg(feature = "serve")]
        let mut serve = None;
        let mut watch = None;
        let mut default_build_path = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let _eq_token: Token![=] = input.parse()?;
            let path: Path = input.parse()?;

            match ident.to_string().as_str() {
                "other_cli_commands" => other_cli_commands = Some(path),
                "pre_build" => pre_build = Some(path),
                "post_build" => post_build = Some(path),
                #[cfg(feature = "serve")]
                "serve" => serve = Some(path),
                #[cfg(not(feature = "serve"))]
                "run_server" => run_server = Some(path),
                "watch" => watch = Some(path),
                "default_build_path" => default_build_path = Some(path),
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
            pre_build,
            post_build,
            #[cfg(feature = "serve")]
            serve,
            watch,
            pkg_name,
            default_build_path,
        })
    }
}
