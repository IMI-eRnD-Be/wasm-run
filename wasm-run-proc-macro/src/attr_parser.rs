use std::collections::HashMap;
use syn::parse::{Error, ParseStream, Result};
use syn::{Ident, Path, Token};

pub struct Attr {
    pub hooks: HashMap<Ident, Path>,
    pub other_cli_commands: Option<Path>,
}

impl Attr {
    pub fn parse(input: ParseStream) -> Result<Self> {
        let mut hooks = HashMap::new();
        let mut other_cli_commands = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let _eq_token: Token![=] = input.parse()?;
            let path: Path = input.parse()?;

            match ident.to_string().as_str() {
                "other_cli_commands" => other_cli_commands = Some(path),
                "prepare_build" | "post_build" | "serve" | "watch" => {
                    if hooks.insert(ident.clone(), path).is_some() {
                        return Err(Error::new(ident.span(), "duplicated key"));
                    }
                }
                _ => return Err(Error::new(ident.span(), "invalid argument")),
            }

            let _comma_token: Token![,] = match input.parse() {
                Ok(x) => x,
                Err(_) if input.is_empty() => break,
                Err(err) => return Err(err),
            };
        }

        Ok(Self {
            hooks,
            other_cli_commands,
        })
    }
}
