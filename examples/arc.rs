extern crate log;
extern crate pretty_env_logger;

use std::path::PathBuf;

use bracket::{
    registry::Registry,
    Result,
};

use serde_json::json;

use bracket_fluent::FluentHelper;
use fluent_templates::ArcLoader;

fn render() -> Result<String> {
    let name = "examples/fluent.md";
    let data = json!({
        "title": "Fluent Example (Arc Loader)",
        //"lang": "en",
        "lang": "fr",
    });

    let mut registry = Registry::new();

    let loader = 
        ArcLoader::builder("examples/locales/", unic_langid::langid!("en"))
            .shared_resources(Some(&["examples/locales/core.ftl".into()]))
            //.customize(|bundle| bundle.set_use_isolating(false))
            .build()
            .unwrap();

    registry
        .helpers_mut()
        .insert("fluent", Box::new(FluentHelper::new(Box::new(loader))));

    registry.load(PathBuf::from(name))?;
    registry.render(name, &data)
}

fn main() {
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();
    match render() {
        Ok(result) => println!("{}", result),
        Err(e) => log::error!("{:?}", e),
    }
}
