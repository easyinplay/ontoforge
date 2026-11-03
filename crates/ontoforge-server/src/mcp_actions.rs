use std::collections::BTreeMap;

use ontoforge_schema::ir::{
    ActionType, BaseType, Ontology, ParamType, PropertyDef, basetype::check_literal,
};
use ontoforge_storage::value::PropertyValue;
use rmcp::model::Tool;
use serde_json::{Map, Value, json};

#[derive(Debug, Clone)]
pub struct ActionToolSchema {
    pub api_name: String,
    pub tool_name: String,
    pub schema: Map<String, Value>,
    validators: BTreeMap<String, ParameterValidator>,
}

#[derive(Debug, Clone)]
enum ParameterValidator {
    Literal(BaseType, Option<BaseType>),
    Reference(PropertyDef),
    Unresolved,
}

impl ActionToolSchema {
    #[must_use]
    pub fn from_action(action: &ActionType, ontology: &Ontology) -> Self {
        let mut properties = Map::new();
        let mut required = Vec::new();
        let mut validators = BTreeMap::new();
        for (name, parameter) in &action.parameters {
            let validator = match &parameter.param_type {
                ParamType::ObjectReference(target) => ontology
                    .object_types
                    .get(target)
                    .and_then(|ty| ty.properties.get(&ty.primary_key))
                    .cloned()
                    .map_or(
                        ParameterValidator::Unresolved,
                        ParameterValidator::Reference,
                    ),
                _ => parameter
                    .param_type
                    .base_type()
                    .map_or(ParameterValidator::Unresolved, |base| {
                        ParameterValidator::Literal(base, parameter.items)
                    }),
            };
            let mut schema = match &validator {
                ParameterValidator::Literal(base, items) => schema_for_type(*base, *items),
                ParameterValidator::Reference(def) => schema_for_type(def.base_type, def.items),
                ParameterValidator::Unresolved => Map::new(),
            };
            validators.insert(name.to_string(), validator);
            if let Some(description) = &parameter.description {
                schema.insert("description".to_owned(), Value::String(description.clone()));
            }
            if let Some(default) = &parameter.default {
                schema.insert("default".to_owned(), default.clone());
            }
            properties.insert(name.to_string(), Value::Object(schema));
            if parameter.required {
                required.push(Value::String(name.to_string()));
            }
        }
        let params_required = !required.is_empty();
        let mut params = json!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
        });
        if params_required {
            params["required"] = Value::Array(required);
        }
        let mut schema = Map::new();
        schema.insert("type".to_owned(), Value::String("object".to_owned()));
        schema.insert(
            "properties".to_owned(),
            json!({
                "commandId": {"type": "string"},
                "validateOnly": {"type": "boolean", "default": false},
                "params": params,
            }),
        );
        if params_required {
            schema.insert("required".to_owned(), json!(["params"]));
        }
        schema.insert("additionalProperties".to_owned(), Value::Bool(false));
        Self {
            api_name: action.api_name.to_string(),
            tool_name: format!("action_{}", action.api_name),
            schema,
            validators,
        }
    }

    #[must_use]
    pub fn tool(&self, description: Option<&str>) -> Tool {
        let description = description.unwrap_or("Ontoforge Action").to_owned();
        Tool::new(self.tool_name.clone(), description, self.schema.clone())
    }

    pub fn validate_arguments(&self, arguments: &Map<String, Value>) -> Result<(), String> {
        if arguments
            .keys()
            .any(|key| !matches!(key.as_str(), "commandId" | "validateOnly" | "params"))
        {
            return Err("unknown action parameter".to_owned());
        }
        if arguments
            .get("commandId")
            .is_some_and(|value| !value.is_string())
            || arguments
                .get("validateOnly")
                .is_some_and(|value| !value.is_boolean())
        {
            return Err("invalid action control parameter".to_owned());
        }
        let empty = Map::new();
        let params = match arguments.get("params") {
            Some(value) => value
                .as_object()
                .ok_or_else(|| "params must be an object".to_owned())?,
            None => &empty,
        };
        if params.keys().any(|key| !self.validators.contains_key(key)) {
            return Err("unknown action parameter".to_owned());
        }
        if let Some(required) = self.schema["properties"]["params"]
            .get("required")
            .and_then(Value::as_array)
            && required
                .iter()
                .filter_map(Value::as_str)
                .any(|key| !params.contains_key(key))
        {
            return Err("missing required action parameter".to_owned());
        }
        for (key, value) in params {
            let valid = self
                .validators
                .get(key)
                .is_some_and(|validator| match validator {
                    ParameterValidator::Literal(base, items) => {
                        check_literal(value, *base, *items).is_ok()
                    }
                    ParameterValidator::Reference(def) => {
                        PropertyValue::from_json(def, value).is_ok()
                    }
                    ParameterValidator::Unresolved => false,
                });
            if !valid {
                return Err(format!("invalid type for action parameter `{key}`"));
            }
        }
        Ok(())
    }
}

fn schema_for_type(base: BaseType, items: Option<BaseType>) -> Map<String, Value> {
    match base {
        BaseType::Byte | BaseType::Short | BaseType::Integer | BaseType::Long => {
            json!({"type":"integer"}).as_object().unwrap().clone()
        }
        BaseType::Float | BaseType::Double => json!({"type":"number"}).as_object().unwrap().clone(),
        BaseType::Decimal => json!({"type":["string","number"]})
            .as_object()
            .unwrap()
            .clone(),
        BaseType::String | BaseType::Date | BaseType::Timestamp | BaseType::Geopoint => {
            json!({"type":"string"}).as_object().unwrap().clone()
        }
        BaseType::Boolean => json!({"type":"boolean"}).as_object().unwrap().clone(),
        BaseType::Array => {
            let item_schema = items.map_or_else(Map::new, |inner| schema_for_type(inner, None));
            json!({"type":"array","items":item_schema})
                .as_object()
                .unwrap()
                .clone()
        }
    }
}
