use syn::parse::{Error, ParseStream, Result};
use syn::{Ident, Path, Token};

pub struct Attr {
    pub other_cli_commands: Option<Path>,
    pub prepare_build: Option<Path>,
    pub post_build: Option<Path>,
    pub serve: Option<Path>,
    pub watch: Option<Path>,
}

impl Attr {
    pub fn parse(input: ParseStream) -> Result<Self> {
        let mut other_cli_commands = None;
        let mut prepare_build = None;
        let mut post_build = None;
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
                "serve" => serve = Some(path),
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
            prepare_build,
            post_build,
            serve,
            watch,
        })
    }
}
