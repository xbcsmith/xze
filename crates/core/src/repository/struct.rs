//! Code structure representations

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete code structure of a repository
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeStructure {
    pub modules: Vec<Module>,
    pub functions: Vec<Function>,
    pub types: Vec<TypeDefinition>,
    pub configs: Vec<ConfigFile>,
}

impl CodeStructure {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total number of items in the structure
    pub fn item_count(&self) -> usize {
        self.modules.len() + self.functions.len() + self.types.len() + self.configs.len()
    }

    /// Check if the structure is empty
    pub fn is_empty(&self) -> bool {
        self.item_count() == 0
    }

    /// Get all public functions
    pub fn public_functions(&self) -> Vec<&Function> {
        self.functions
            .iter()
            .filter(|f| f.visibility == Visibility::Public)
            .collect()
    }
}

/// Module representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub documentation: Option<String>,
    pub visibility: Visibility,
}

/// Function representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub signature: String,
    pub documentation: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub is_async: bool,
}

/// Function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: String,
    pub default_value: Option<String>,
}

/// Type definition (struct, enum, trait, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    pub documentation: Option<String>,
    pub fields: Vec<Field>,
    pub visibility: Visibility,
}

/// Kind of type definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    Interface,
    Class,
}

/// Field in a type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub type_annotation: String,
    pub documentation: Option<String>,
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

/// Configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub format: ConfigFormat,
    pub content: String,
}

/// Configuration file format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    Yaml,
    Toml,
    Json,
    Env,
}

impl ConfigFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            "json" => Some(Self::Json),
            "env" => Some(Self::Env),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_structure() {
        let mut structure = CodeStructure::new();
        assert!(structure.is_empty());
        assert_eq!(structure.item_count(), 0);

        structure.functions.push(Function {
            name: "test_fn".to_string(),
            signature: "fn test_fn()".to_string(),
            documentation: None,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
        });

        assert!(!structure.is_empty());
        assert_eq!(structure.item_count(), 1);
    }

    #[test]
    fn test_public_functions() {
        let mut structure = CodeStructure::new();

        structure.functions.push(Function {
            name: "public_fn".to_string(),
            signature: "pub fn public_fn()".to_string(),
            documentation: None,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
        });

        structure.functions.push(Function {
            name: "private_fn".to_string(),
            signature: "fn private_fn()".to_string(),
            documentation: None,
            parameters: vec![],
            return_type: None,
            visibility: Visibility::Private,
            is_async: false,
        });

        assert_eq!(structure.public_functions().len(), 1);
    }

    #[test]
    fn test_config_format() {
        assert_eq!(
            ConfigFormat::from_extension("yaml"),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_extension("toml"),
            Some(ConfigFormat::Toml)
        );
        assert_eq!(
            ConfigFormat::from_extension("json"),
            Some(ConfigFormat::Json)
        );
        assert_eq!(ConfigFormat::from_extension("unknown"), None);
    }
}
