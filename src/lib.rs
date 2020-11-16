//! Helper for fluent language lookup.
use std::collections::HashMap;
use bracket::{
    error::HelperError,
    helper::{Helper, HelperValue},
    parser::ast::Node,
    render::{Context, Render, Type},
};

use serde_json::Value;

use fluent_templates::Loader;
use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::LanguageIdentifier;

/// Lookup a language string in the underlying loader.
pub struct Fluent {
    loader: Box<dyn Loader + Send + Sync>,
}

impl Fluent {
    pub fn new(loader: Box<dyn Loader + Send + Sync>) -> Self {
        Self {loader} 
    }
}

impl Helper for Fluent {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        _template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(1..usize::MAX)?;

        let msg_id = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

        let lang = rc
            .evaluate("@root.lang")?
            .ok_or_else(|| {
                HelperError::Message(
                    format!("Helper '{}' requires a 'lang' variable in the root data", ctx.name()))
            })?
            .as_str()
            .ok_or_else(|| {
                HelperError::Message(
                    format!("Helper '{}' requires that the 'lang' variable is a string", ctx.name()))
            })?;

        // Build arguments from hash parameters
        let args: Option<HashMap<String, FluentValue>> = if ctx.parameters().is_empty() {
            None
        } else {
            let map = ctx
                .parameters()
                .iter()
                .filter_map(|(k, v)| {
                    let val = match v {
                        // `Number::as_f64` can't fail here because we haven't
                        // enabled `arbitrary_precision` feature
                        // in `serde_json`.
                        Value::Number(n) => n.as_f64().unwrap().into(),
                        Value::String(s) => s.to_owned().into(),
                        _ => return None,
                    };
                    Some((k.to_string(), val))
                })
                .collect();
            Some(map)
        };

        let lang_id = lang.parse::<LanguageIdentifier>()
            .map_err(|e| HelperError::Message(e.to_string()))?;

        let message = self.loader.lookup_complete(&lang_id, &msg_id, args.as_ref());
        rc.write_escaped(&message)?;

        Ok(None)
    }
}
