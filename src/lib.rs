//! Helper for fluent language lookup.
use std::borrow::Cow;
use std::collections::HashMap;

use bracket::{
    error::HelperError,
    helper::{Helper, HelperValue},
    parser::ast::{Node, ParameterValue},
    render::{Context, Render, Type},
};

use serde_json::Value;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::LanguageIdentifier;
use fluent_templates::Loader;

/// Lookup a language string in the underlying loader.
pub struct Fluent {
    loader: Box<dyn Loader + Send + Sync>,
    trim: bool,
}

impl Fluent {

    /// Create a new fluent helper.
    ///
    /// Messages are resolved using the underlying loader.
    ///
    /// When `trim` is `true` then parameters passed in using `fluentparam` child 
    /// blocks have leading and trailing whitespace removed.
    pub fn new(loader: Box<dyn Loader + Send + Sync>, trim: bool) -> Self {
        Self { loader, trim }
    }
}

impl Helper for Fluent {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(1..usize::MAX)?;

        let msg_id = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

        let lang = rc
            .evaluate("@root.lang")?
            .ok_or_else(|| {
                HelperError::Message(format!(
                    "Helper '{}' requires a 'lang' variable in the root data",
                    ctx.name()
                ))
            })?
            .as_str()
            .ok_or_else(|| {
                HelperError::Message(format!(
                    "Helper '{}' requires that the 'lang' variable is a string",
                    ctx.name()
                ))
            })?;

        // Build arguments from hash parameters
        let mut args: Option<HashMap<String, FluentValue>> =
            if ctx.parameters().is_empty() {
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

        if let Some(node) = template {
            for child in node.into_iter() {
                match child {
                    Node::Block(ref block) => {
                        if let Some("fluentparam") = block.name() {
                            let content = rc.buffer(child)?;
                            let param = block.call().arguments().get(0).ok_or_else(|| {
                                HelperError::Message(
                                    format!("Block 'fluentparam' must have a single argument")
                                )
                            })?;

                            if let ParameterValue::Json(ref value) = param {
                                if let Value::String(ref s) = value {
                                    let params =
                                        args.get_or_insert(HashMap::new());

                                    let value = if self.trim {
                                        Cow::from(content.trim().to_string())
                                    } else {
                                        Cow::from(content)
                                    };

                                    println!("Parameter value {:?}", value);

                                    params.insert(
                                        s.to_string(),
                                        FluentValue::String(value),
                                    );
                                } else {
                                    return Err(HelperError::Message(
                                        format!("Block 'fluentparam' expects a string argument")
                                    ));
                                }
                            } else {
                                return Err(HelperError::Message(
                                    format!("Block 'fluentparam' expects a JSON literal argument")
                                ));
                            }
                        } else {
                            return Err(HelperError::Message(format!(
                                "Helper '{}' only allows 'fluentparam' blocks",
                                ctx.name()
                            )));
                        }
                    }
                    _ => {}
                }
            }
        }

        let lang_id = lang
            .parse::<LanguageIdentifier>()
            .map_err(|e| HelperError::Message(e.to_string()))?;

        let message =
            self.loader
                .lookup_complete(&lang_id, &msg_id, args.as_ref());
        rc.write_escaped(&message)?;

        Ok(None)
    }
}
