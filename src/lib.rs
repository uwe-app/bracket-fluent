#![deny(missing_docs)]

//! Helper for [fluent](https://www.projectfluent.org/) language lookup.
//!
//! The root data for the template must contain a `lang` field 
//! with the identifier of the current language.
//!
//! Assuming a fluent language file in `locales/en/main.ftl` and the `lang` 
//! field is set to `en`, using the contents:
//!
//! ```ignore
//! welcome = Hello!
//! greeting = Hello { $name }!
//! block = { $var1 } { $var2 }
//! ```
//!
//! We can resolve the message in a template using the fluent helper like this:
//!
//! ```ignore
//! {{fluent "welcome"}}
//! ```
//!
//! Pass variables using the helper hash parameters:
//!
//! ```ignore
//! {{fluent "greeting" name="world"}}
//! ```
//!
//! If you need to pass multi-line variables to a message use the `fluentparam` syntax 
//! inside a block call:
//!
//! ```ignore
//! {{#fluent "block"~}}
//! {{#fluentparam "var1"~}}
//! This is some multi-line content for 
//! the first variable parameter named var1.
//! {{/fluentparam}}
//!
//! {{#fluentparam "var2"}}
//! Which is continued in another multi-line 
//! paragraph using the variable named var2.
//! {{~/fluentparam~}}
//! {{/fluent}}
//! ```
//!
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bracket::{
    error::HelperError,
    helper::{Helper, HelperValue, LocalHelper},
    parser::ast::Node,
    render::{Context, Render, Type},
};

use serde_json::Value;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::LanguageIdentifier;
use fluent_templates::Loader;

static FLUENT_PARAM: &str = "fluentparam";

/// Local helper for `{{#fluentparam}}` blocks.
#[derive(Clone)]
pub struct FluentParam {
    parameters: Arc<RwLock<HashMap<String, String>>>,
}

impl Helper for FluentParam {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(1..1)?;
        ctx.assert_block(template)?;

        let param_name = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

        let node = template.unwrap();
        let content = rc.buffer(node)?;
        let mut writer = self.parameters.write().unwrap();
        writer.insert(param_name.to_string(), content);

        Ok(None)
    }
}

impl LocalHelper for FluentParam {}

/// Lookup a language string in the underlying loader.
pub struct FluentHelper {
    loader: Box<dyn Loader + Send + Sync>,
    /// Escape messages, default is `true`.
    pub escape: bool,
}

impl FluentHelper {
    /// Create a new fluent helper.
    ///
    /// Messages are resolved using the underlying loader.
    pub fn new(loader: Box<dyn Loader + Send + Sync>) -> Self {
        Self {
            loader,
            escape: true,
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
        ctx.arity(1..1)?;

        let msg_id = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

        let lang = rc
            .evaluate("@root.lang")?
            .ok_or_else(|| {
                HelperError::new(format!(
                    "Helper '{}' requires a 'lang' variable in the template data",
                    ctx.name()
                ))
            })?
            .as_str()
            .ok_or_else(|| {
                HelperError::new(format!(
                    "Type error in helper '{}' the 'lang' variable must be a string",
                    ctx.name()
                ))
            })?;

        let lang_id = lang
            .parse::<LanguageIdentifier>()
            .map_err(|e| HelperError::new(e.to_string()))?;

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
            let parameters: Arc<RwLock<HashMap<String, String>>> =
                Arc::new(RwLock::new(HashMap::new()));
            let local_helper = FluentParam {
                parameters: Arc::clone(&parameters),
            };
            rc.register_local_helper(FLUENT_PARAM, Box::new(local_helper));
            rc.template(node)?;
            rc.unregister_local_helper(FLUENT_PARAM);

            let lock = Arc::try_unwrap(parameters).expect(
                "Fluent helper parameters lock still has multiple owners!",
            );
            let map = lock
                .into_inner()
                .expect("Fluent helper failed to get inner value from lock!");

            let params = args.get_or_insert(HashMap::new());
            for (k, v) in map {
                params.insert(k, FluentValue::String(v.into()));
            }
        }

        let message =
            self.loader
                .lookup_complete(&lang_id, &msg_id, args.as_ref());
        if self.escape {
            rc.write_escaped(&message)?;
        } else {
            rc.write(&message)?;
        }

        Ok(None)
    }
}
