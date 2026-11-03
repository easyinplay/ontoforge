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
        properties.insert(
            "validateOnly".to_owned(),
            json!({"type":"boolean","default":false}),
        );
        properties.insert("commandId".to_owned(), json!({"type":"string"}));
        validators.insert(
            "validateOnly".to_owned(),
            ParameterValidator::Literal(BaseType::Boolean, None),
        );
        validators.insert(
            "commandId".to_owned(),
            ParameterValidator::Literal(BaseType::String, None),
        );
        let mut schema = Map::new();
        schema.insert("type".to_owned(), Value::String("object".to_owned()));
        schema.insert("properties".to_owned(), Value::Object(properties));
        if !required.is_empty() {
            schema.insert("required".to_owned(), Value::Array(required));
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
        let Some(properties) = self.schema.get("properties").and_then(Value::as_object) else {
            return Ok(());
        };
        if arguments.keys().any(|key| !properties.contains_key(key)) {
            return Err("unknown action parameter".to_owned());
        }
        if let Some(required) = self.schema.get("required").and_then(Value::as_array)
            && required
                .iter()
                .filter_map(Value::as_str)
                .any(|key| !arguments.contains_key(key))
        {
            return Err("missing required action parameter".to_owned());
        }
        for (key, value) in arguments {
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
