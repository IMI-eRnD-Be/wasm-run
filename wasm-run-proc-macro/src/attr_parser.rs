use syn::parse::{Error, ParseStream, Result};
use syn::{Ident, LitStr, Path, Token};

pub struct Attr {
    pub other_cli_commands: Option<Path>,
    pub pre_build: Option<Path>,
    pub post_build: Option<Path>,
    #[cfg(feature = "mini-http-server")]
    pub serve: Option<Path>,
    pub watch: Option<Path>,
    pub pkg_name: Option<LitStr>,
    pub backend_pkg_name: Option<LitStr>,
    pub default_build_path: Option<Path>,
    pub build_args: Option<Path>,
    pub serve_args: Option<Path>,
}

impl Attr {
    pub fn parse(input: ParseStream) -> Result<Self> {
        let pkg_name = input.parse().ok();

        if pkg_name.is_some() && !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        let backend_pkg_name = input.parse().ok();

        if backend_pkg_name.is_some() && !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        let mut other_cli_commands = None;
        let mut pre_build = None;
        let mut post_build = None;
        #[cfg(feature = "mini-http-server")]
        let mut serve = None;
        let mut watch = None;
        let mut default_build_path = None;
        let mut build_args = None;
        let mut serve_args = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let path: Path = if input.parse::<Token![=]>().is_ok() {
                input.parse()?
            } else {
                ident.clone().into()
            };

            match ident.to_string().as_str() {
                "other_cli_commands" => other_cli_commands = Some(path),
                "pre_build" => pre_build = Some(path),
                "post_build" => post_build = Some(path),
                #[cfg(feature = "mini-http-server")]
                "serve" => serve = Some(path),
                "watch" => watch = Some(path),
                "default_build_path" => default_build_path = Some(path),
                "build_args" => build_args = Some(path),
                "serve_args" => serve_args = Some(path),
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
        })
    }
}
