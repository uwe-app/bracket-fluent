//! Helper for fluent language lookup.
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

static FLUENT_PARAM: &str = "fluentparam";

/// Lookup a language string in the underlying loader.
pub struct FluentHelper {
    loader: Box<dyn Loader + Send + Sync>,
    escaped: bool,
    trim: bool,
}

impl FluentHelper {
    /// Create a new fluent helper.
    ///
    /// Messages are resolved using the underlying loader.
    ///
    /// When `trim` is `true` then parameters passed in using `fluentparam` child
    /// blocks have leading and trailing whitespace removed.
    pub fn new(
        loader: Box<dyn Loader + Send + Sync>,
        escaped: bool,
        trim: bool,
    ) -> Self {
        Self {
            loader,
            escaped,
            trim,
        }
    }
}

impl Helper for FluentHelper {
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
                    "Helper '{}' requires a 'lang' variable in the template data",
                    ctx.name()
                ))
            })?
            .as_str()
            .ok_or_else(|| {
                HelperError::Message(format!(
                    "Type error in helper '{}' the 'lang' variable must be a string",
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
                                HelperError::new(
                                    format!("Block '{}' must have a single argument", FLUENT_PARAM)
                                )
                            })?;

                            if let ParameterValue::Json(ref value) = param {
                                if let Value::String(ref s) = value {
                                    let params =
                                        args.get_or_insert(HashMap::new());

                                    let value = if self.trim {
                                        content.trim().to_string()
                                    } else {
                                        content
                                    };

                                    params.insert(
                                        s.to_string(),
                                        FluentValue::String(value.into()),
                                    );
                                } else {
                                    return Err(HelperError::new(
                                        format!("Block '{}' expects a string argument", FLUENT_PARAM)
                                    ));
                                }
                            } else {
                                return Err(HelperError::new(
                                    format!("Block '{}' expects a JSON literal argument", FLUENT_PARAM)
                                ));
                            }
                        } else {
                            return Err(HelperError::new(format!(
                                "Helper '{}' only allows '{}' blocks",
                                ctx.name(), FLUENT_PARAM
                            )));
                        }
                    }
                    _ => {}
                }
            }
        }

        let lang_id = lang
            .parse::<LanguageIdentifier>()
            .map_err(|e| HelperError::new(e.to_string()))?;

        let message =
            self.loader
                .lookup_complete(&lang_id, &msg_id, args.as_ref());
        if self.escaped {
            rc.write_escaped(&message)?;
        } else {
            rc.write(&message)?;
        }

        Ok(None)
    }
}
