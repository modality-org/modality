use serde_json::{Value, Map};
use regex::Regex;

#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct Options {
    space: Option<String>,
    cycles: bool,
    replacer: Option<fn(&str, &Value) -> Option<Value>>,
    stringify: fn(&Value) -> String,
    compare: Option<fn(&str, &Value, &str, &Value) -> std::cmp::Ordering>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            space: None,
            cycles: true,
            replacer: None,
            stringify: |v| serde_json::to_string(v).unwrap(),
            compare: None,
        }
    }
}

pub fn serialize(value: &Value) -> Value {
  match value {
      Value::Null => Value::Null,
      Value::Bool(_) => value.clone(),
      Value::Number(_) => value.clone(),
      Value::String(s) => {
          if let Ok(re) = Regex::new(s) {
              Value::String(re.to_string())
          } else {
              value.clone()
          }
      },
      Value::Array(arr) => Value::Array(arr.iter().map(serialize).collect()),
      Value::Object(obj) => {
          if obj.contains_key("toJSON") {
              // In Rust, we can't directly call a method on a JSON object
              // This is a placeholder for the JavaScript `obj.toJSON()` functionality
              // You might want to implement a custom logic here
              value.clone()
          } else {
              Value::Object(obj.iter().map(|(k, v)| (k.clone(), serialize(v))).collect())
          }
      },
  }
}

pub fn stringify_deterministic(obj: &Value, opts: Option<Options>) -> String {
    let opts = opts.unwrap_or_default();
    
    // Detect circular structure
    if !opts.cycles {
        (opts.stringify)(obj);
    }

    let mut seen = Vec::new();

    fn deterministic(
        _parent: &Value,
        key: &str,
        node: &Value,
        level: usize,
        opts: &Options,
        seen: &mut Vec<*const Value>,
    ) -> Option<String> {
        let indent = opts.space.as_ref().map(|s| "\n".to_string() + &s.repeat(level + 1)).unwrap_or_default();
        let colon_separator = if opts.space.is_some() { ": " } else { ":" };

        let mut node = serialize(node);
        if let Some(replacer) = opts.replacer {
            node = replacer(key, &node).unwrap_or(node);
        }

        if node == Value::Null {
            return None;
        }

        match &node {
          Value::Null => Some((opts.stringify)(&node)),
          Value::Bool(_) | Value::Number(_) | Value::String(_) => Some((opts.stringify)(&node)),
          Value::Array(arr) => {
              let mut out = Vec::new();
              for (i, item) in arr.iter().enumerate() {
                  let value = deterministic(
                      &node,
                      &i.to_string(),
                      item,
                      level + 1,
                      opts,
                      seen,
                  ).unwrap_or_else(|| (opts.stringify)(&Value::Null));
                  out.push(format!("{}{}{}", indent, opts.space.as_ref().unwrap_or(&"".to_string()), value));
              }
              Some(format!("[{}{}]", out.join(","), indent))
          },
          Value::Object(obj) => {
              if opts.cycles {
                  let node_ptr = &node as *const Value;
                  if seen.contains(&node_ptr) {
                      return Some((opts.stringify)(&Value::String("[Circular]".to_string())));
                  } else {
                      seen.push(node_ptr);
                  }
              }
  
              let mut node_keys: Vec<&String> = obj.keys().collect();
              if let Some(compare) = &opts.compare {
                  node_keys.sort_by(|a, b| compare(a, obj.get(*a).unwrap(), b, obj.get(*b).unwrap()));
              } else {
                  node_keys.sort();
              }
  
              let mut out = Vec::new();
              for key in node_keys {
                let value = deterministic(
                    &node,
                    key,
                    obj.get(key).unwrap(),
                    level + 1,
                    opts,
                    seen,
                ).unwrap_or_else(|| (opts.stringify)(&Value::Null));  // Changed this line to handle null values
                let key_value = format!("{}{}{}", (opts.stringify)(&Value::String(key.to_string())), colon_separator, value);
                out.push(format!("{}{}{}", indent, opts.space.as_ref().unwrap_or(&"".to_string()), key_value));
              }
  
              if opts.cycles {
                  seen.pop();
              }
  
              Some(format!("{{{}{}}}", out.join(","), indent))
          },
        }
    }

    deterministic(
        &Value::Object(Map::new()),
        "",
        obj,
        0,
        &opts,
        &mut seen,
    ).unwrap_or_default()
}