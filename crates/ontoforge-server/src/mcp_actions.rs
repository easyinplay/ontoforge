use ontoforge_schema::ir::{ActionType, ParamType};
use rmcp::model::Tool;
use serde_json::{Map, Value, json};

#[derive(Debug, Clone)]
pub struct ActionToolSchema {
    pub api_name: String,
    pub tool_name: String,
    pub schema: Map<String, Value>,
}

impl ActionToolSchema {
    #[must_use]
    pub fn from_action(action: &ActionType) -> Self {
        let mut properties = Map::new();
        let mut required = Vec::new();
        for (name, parameter) in &action.parameters {
            let mut schema = schema_for_param(&parameter.param_type, parameter.items.as_ref());
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
        Ok(())
    }
}

fn schema_for_param(
    param_type: &ParamType,
    items: Option<&ontoforge_schema::ir::BaseType>,
) -> Map<String, Value> {
    match param_type {
        ParamType::Byte | ParamType::Short | ParamType::Integer | ParamType::Long => {
            json!({"type":"integer"}).as_object().unwrap().clone()
        }
        ParamType::Float | ParamType::Double => {
            json!({"type":"number"}).as_object().unwrap().clone()
        }
        ParamType::Decimal
        | ParamType::String
        | ParamType::Date
        | ParamType::Timestamp
        | ParamType::Geopoint
        | ParamType::ObjectReference(_) => json!({"type":"string"}).as_object().unwrap().clone(),
        ParamType::Boolean => json!({"type":"boolean"}).as_object().unwrap().clone(),
        ParamType::Array => {
            let item_schema = match items {
                Some(
                    ontoforge_schema::ir::BaseType::Integer
                    | ontoforge_schema::ir::BaseType::Long
                    | ontoforge_schema::ir::BaseType::Short
                    | ontoforge_schema::ir::BaseType::Byte,
                ) => json!({"type":"integer"}),
                Some(ontoforge_schema::ir::BaseType::Boolean) => json!({"type":"boolean"}),
                _ => json!({"type":"string"}),
            };
            json!({"type":"array","items":item_schema})
                .as_object()
                .unwrap()
                .clone()
        }
    }
}
