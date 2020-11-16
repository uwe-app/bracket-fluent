extern crate log;
extern crate pretty_env_logger;

use std::convert::TryFrom;
use std::path::PathBuf;

use bracket::{
    registry::Registry,
    template::{Loader, Templates},
    Result,
};

use serde_json::json;

use bracket_fluent::FluentHelper;

fluent_templates::static_loader! {
    static LOCALES = {
        locales: "./examples/locales",
        fallback_language: "en",
        core_locales: "./examples/locales/core.ftl",
    };
}

fn render() -> Result<String> {
    let name = "examples/fluent.md";
    let data = json!({
        "title": "Fluent Example (Static Loader)",
        "lang": "en",
        //"lang": "fr",
    });

    let mut loader = Loader::new();
    loader.load(PathBuf::from(name))?;

    let templates = Templates::try_from(&loader)?;
    let mut registry = Registry::from(templates);

    registry
        .helpers_mut()
        .insert("fluent", Box::new(FluentHelper::new(Box::new(&*LOCALES))));

    registry.render(name, &data)
}

fn main() {
    std::env::set_var("RUST_LOG", "trace");
    pretty_env_logger::init();
    match render() {
        Ok(result) => println!("{}", result),
        Err(e) => log::error!("{:?}", e),
    }
}
