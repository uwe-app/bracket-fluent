# {{title}}

Fluent helper example.

## Language Identifier

> {{lang}}

## Example Message

> {{fluent "welcome"}}

## Example Parameters Block

{{#fluent "block"}}
{{#fluentparam "var1"}}
This is some multi-line content for 
the first variable parameter named `var1`.
{{/fluentparam}}

{{#fluentparam "var2"}}
Which is continued in another multi-line 
paragraph using the variable named `var2`.
{{/fluentparam}}
{{/fluent}}

## Fallback Message

> {{fluent "fallback-message"}}

## Variable Interpolation

> {{fluent "interpolated-message" var="FOO"}}

## Unknown Localization

> {{fluent "non-existent-message-id"}}
