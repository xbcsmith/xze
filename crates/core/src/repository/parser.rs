//! Code parser for detailed syntax analysis

use crate::{
    error::{Result, XzeError},
    repository::{Field, Function, Parameter, TypeDefinition},
    types::ProgrammingLanguage,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Code parser trait for detailed syntax analysis
pub trait CodeParser: Send + Sync {
    /// Parse a source file and extract detailed structure
    fn parse_file(&self, content: &str, file_path: &Path) -> Result<ParseResult>;

    /// Parse function signatures and extract parameters
    fn parse_function_signature(&self, signature: &str) -> Result<Vec<Parameter>>;

    /// Parse type definitions and extract fields
    fn parse_type_fields(&self, content: &str, type_name: &str) -> Result<Vec<Field>>;

    /// Extract documentation comments
    fn extract_documentation(&self, content: &str, line_number: usize) -> Option<String>;
}

/// Result of parsing a source file
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParseResult {
    /// Functions found in the file
    pub functions: Vec<Function>,
    /// Type definitions found in the file
    pub types: Vec<TypeDefinition>,
    /// Imports/dependencies found
    pub imports: Vec<Import>,
    /// Exports found (for languages that support them)
    pub exports: Vec<Export>,
    /// Comments and documentation
    pub documentation: Vec<DocumentationBlock>,
}

impl ParseResult {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            types: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            documentation: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: ParseResult) {
        self.functions.extend(other.functions);
        self.types.extend(other.types);
        self.imports.extend(other.imports);
        self.exports.extend(other.exports);
        self.documentation.extend(other.documentation);
    }
}

/// Import statement representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    /// Module or package being imported
    pub module: String,
    /// Specific items imported (if applicable)
    pub items: Vec<String>,
    /// Alias for the import (if any)
    pub alias: Option<String>,
    /// Whether this is a wildcard import
    pub is_wildcard: bool,
}

/// Export statement representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    /// Name of the exported item
    pub name: String,
    /// Type of export (function, class, variable, etc.)
    pub export_type: ExportType,
    /// Whether this is a default export
    pub is_default: bool,
}

/// Type of export
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportType {
    Function,
    Class,
    Interface,
    Type,
    Variable,
    Constant,
    Module,
}

/// Documentation block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationBlock {
    /// The documentation content
    pub content: String,
    /// Line number where the documentation starts
    pub line_start: usize,
    /// Line number where the documentation ends
    pub line_end: usize,
    /// What this documentation is for
    pub target: Option<String>,
}

/// Factory for creating code parsers
pub struct ParserFactory;

impl ParserFactory {
    /// Create a parser for the given language
    pub fn create_parser(language: &ProgrammingLanguage) -> Box<dyn CodeParser> {
        match language {
            ProgrammingLanguage::Rust => Box::new(RustParser::new()),
            ProgrammingLanguage::Go => Box::new(GoParser::new()),
            ProgrammingLanguage::Python => Box::new(PythonParser::new()),
            ProgrammingLanguage::JavaScript => Box::new(JavaScriptParser::new()),
            ProgrammingLanguage::TypeScript => Box::new(TypeScriptParser::new()),
            ProgrammingLanguage::Java => Box::new(JavaParser::new()),
            _ => Box::new(GenericParser::new()),
        }
    }
}

/// Rust code parser
#[derive(Debug, Default)]
pub struct RustParser;

impl RustParser {
    pub fn new() -> Self {
        Self
    }

    fn parse_rust_function_params(&self, signature: &str) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();

        // Find parameter list between parentheses
        let start = signature.find('(').ok_or_else(|| {
            XzeError::validation("Invalid function signature: missing opening parenthesis")
        })?;

        let end = signature.rfind(')').ok_or_else(|| {
            XzeError::validation("Invalid function signature: missing closing parenthesis")
        })?;

        if start >= end {
            return Ok(params);
        }

        let params_str = &signature[start + 1..end];
        if params_str.trim().is_empty() {
            return Ok(params);
        }

        // Split by comma, but be careful about nested generics
        let param_parts = self.split_rust_params(params_str);

        for param_part in param_parts {
            let param_part = param_part.trim();
            if param_part.is_empty() || param_part == "self" || param_part.starts_with("&self") {
                continue;
            }

            // Parse "name: type" or "name: type = default"
            if let Some(colon_pos) = param_part.find(':') {
                let name = param_part[..colon_pos].trim().to_string();
                let rest = &param_part[colon_pos + 1..];

                let (type_annotation, default_value) = if let Some(eq_pos) = rest.find('=') {
                    let type_part = rest[..eq_pos].trim();
                    let default_part = rest[eq_pos + 1..].trim();
                    (type_part.to_string(), Some(default_part.to_string()))
                } else {
                    (rest.trim().to_string(), None)
                };

                params.push(Parameter {
                    name,
                    type_annotation,
                    default_value,
                });
            }
        }

        Ok(params)
    }

    fn split_rust_params(&self, params_str: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for ch in params_str.chars() {
            if escape_next {
                escape_next = false;
                current.push(ch);
                continue;
            }

            match ch {
                '\\' if in_string => {
                    escape_next = true;
                    current.push(ch);
                }
                '"' => {
                    in_string = !in_string;
                    current.push(ch);
                }
                '<' | '(' | '[' if !in_string => {
                    depth += 1;
                    current.push(ch);
                }
                '>' | ')' | ']' if !in_string => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if !in_string && depth == 0 => {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }

        parts
    }

    fn parse_rust_struct_fields(&self, content: &str, struct_name: &str) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_struct = false;
        let mut brace_count = 0;

        for line in lines {
            let trimmed = line.trim();

            // Find struct definition
            if trimmed.contains(&format!("struct {}", struct_name)) {
                in_struct = true;
                if trimmed.contains('{') {
                    brace_count += 1;
                }
                continue;
            }

            if !in_struct {
                continue;
            }

            // Count braces to know when we're done with the struct
            brace_count += trimmed.chars().filter(|&c| c == '{').count();
            brace_count -= trimmed.chars().filter(|&c| c == '}').count();

            if brace_count == 0 {
                break;
            }

            // Parse field: "pub field_name: FieldType,"
            if trimmed.contains(':') && !trimmed.starts_with("//") {
                let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let field_name = parts[0].trim().trim_start_matches("pub").trim().to_string();

                    let field_type = parts[1].trim().trim_end_matches(',').to_string();

                    if !field_name.is_empty() && !field_type.is_empty() {
                        fields.push(Field {
                            name: field_name,
                            type_annotation: field_type,
                            documentation: None, // TODO: Extract field documentation
                        });
                    }
                }
            }
        }

        Ok(fields)
    }
}

impl CodeParser for RustParser {
    fn parse_file(&self, content: &str, _file_path: &Path) -> Result<ParseResult> {
        let mut result = ParseResult::new();

        // This is a simplified parser - in a real implementation,
        // you might want to use a proper Rust parser like syn

        let lines: Vec<&str> = content.lines().collect();

        for line in lines.iter() {
            let trimmed = line.trim();

            // Parse use statements
            if trimmed.starts_with("use ") {
                if let Some(import) = self.parse_use_statement(trimmed) {
                    result.imports.push(import);
                }
            }

            // Parse functions (basic detection)
            if trimmed.contains("fn ") && !trimmed.starts_with("//") {
                // This would be enhanced to properly parse the full function
                continue;
            }
        }

        Ok(result)
    }

    fn parse_function_signature(&self, signature: &str) -> Result<Vec<Parameter>> {
        self.parse_rust_function_params(signature)
    }

    fn parse_type_fields(&self, content: &str, type_name: &str) -> Result<Vec<Field>> {
        self.parse_rust_struct_fields(content, type_name)
    }

    fn extract_documentation(&self, content: &str, line_number: usize) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut doc_lines = Vec::new();
        let mut current_line = line_number;

        // Look backwards for doc comments
        while current_line > 0 {
            current_line -= 1;
            if let Some(line) = lines.get(current_line) {
                let trimmed = line.trim();
                if trimmed.starts_with("///") {
                    doc_lines.insert(0, trimmed.trim_start_matches("///").trim());
                } else if trimmed.starts_with("//!") {
                    doc_lines.insert(0, trimmed.trim_start_matches("//!").trim());
                } else if trimmed.is_empty() {
                    continue;
                } else {
                    break;
                }
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }
}

impl RustParser {
    fn parse_use_statement(&self, line: &str) -> Option<Import> {
        // Parse "use module::item;" or "use module::*;" etc.
        let use_content = line.strip_prefix("use ")?.strip_suffix(';')?;

        if use_content.ends_with("::*") {
            let module = use_content.strip_suffix("::*")?.to_string();
            Some(Import {
                module,
                items: Vec::new(),
                alias: None,
                is_wildcard: true,
            })
        } else if use_content.contains(" as ") {
            let parts: Vec<&str> = use_content.split(" as ").collect();
            if parts.len() == 2 {
                Some(Import {
                    module: parts[0].to_string(),
                    items: Vec::new(),
                    alias: Some(parts[1].to_string()),
                    is_wildcard: false,
                })
            } else {
                None
            }
        } else {
            Some(Import {
                module: use_content.to_string(),
                items: Vec::new(),
                alias: None,
                is_wildcard: false,
            })
        }
    }
}

/// Go code parser
#[derive(Debug, Default)]
pub struct GoParser;

impl GoParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for GoParser {
    fn parse_file(&self, content: &str, _file_path: &Path) -> Result<ParseResult> {
        let result = ParseResult::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Parse imports
            if trimmed.starts_with("import ") {
                // Handle both single imports and import blocks
                continue;
            }
        }

        Ok(result)
    }

    fn parse_function_signature(&self, _signature: &str) -> Result<Vec<Parameter>> {
        // Go function parameter parsing
        Ok(Vec::new()) // Simplified
    }

    fn parse_type_fields(&self, _content: &str, _type_name: &str) -> Result<Vec<Field>> {
        // Go struct field parsing
        Ok(Vec::new()) // Simplified
    }

    fn extract_documentation(&self, _content: &str, _line_number: usize) -> Option<String> {
        // Go documentation comment extraction
        None // Simplified
    }
}

/// Python code parser
#[derive(Debug, Default)]
pub struct PythonParser;

impl PythonParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for PythonParser {
    fn parse_file(&self, content: &str, _file_path: &Path) -> Result<ParseResult> {
        let mut result = ParseResult::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Parse imports
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                if let Some(import) = self.parse_python_import(trimmed) {
                    result.imports.push(import);
                }
            }
        }

        Ok(result)
    }

    fn parse_function_signature(&self, signature: &str) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();

        // Find parameter list
        if let Some(start) = signature.find('(') {
            if let Some(end) = signature.rfind(')') {
                let params_str = &signature[start + 1..end];

                for param in params_str.split(',') {
                    let param = param.trim();
                    if param.is_empty() || param == "self" {
                        continue;
                    }

                    let (name, type_annotation, default_value) = if param.contains(':') {
                        let parts: Vec<&str> = param.splitn(2, ':').collect();
                        let name = parts[0].trim();
                        let type_part = parts[1].trim();

                        if type_part.contains('=') {
                            let type_parts: Vec<&str> = type_part.splitn(2, '=').collect();
                            (name, Some(type_parts[0].trim()), Some(type_parts[1].trim()))
                        } else {
                            (name, Some(type_part), None)
                        }
                    } else if param.contains('=') {
                        let parts: Vec<&str> = param.splitn(2, '=').collect();
                        (parts[0].trim(), None, Some(parts[1].trim()))
                    } else {
                        (param, None, None)
                    };

                    params.push(Parameter {
                        name: name.to_string(),
                        type_annotation: type_annotation.unwrap_or("Any").to_string(),
                        default_value: default_value.map(|s| s.to_string()),
                    });
                }
            }
        }

        Ok(params)
    }

    fn parse_type_fields(&self, _content: &str, _type_name: &str) -> Result<Vec<Field>> {
        // Python class attribute parsing (simplified)
        Ok(Vec::new())
    }

    fn extract_documentation(&self, content: &str, line_number: usize) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();

        // Look for docstring after the line
        if let Some(next_line_idx) = line_number.checked_add(1) {
            if next_line_idx < lines.len() {
                let mut current_line = next_line_idx;

                // Skip empty lines
                while current_line < lines.len() && lines[current_line].trim().is_empty() {
                    current_line += 1;
                }

                if current_line < lines.len() {
                    let line = lines[current_line].trim();
                    if line.starts_with("\"\"\"") || line.starts_with("'''") {
                        return self.extract_python_docstring(&lines, current_line);
                    }
                }
            }
        }

        None
    }
}

impl PythonParser {
    fn parse_python_import(&self, line: &str) -> Option<Import> {
        if line.starts_with("import ") {
            let import_part = line.strip_prefix("import ")?;
            if import_part.contains(" as ") {
                let parts: Vec<&str> = import_part.split(" as ").collect();
                if parts.len() == 2 {
                    Some(Import {
                        module: parts[0].to_string(),
                        items: Vec::new(),
                        alias: Some(parts[1].to_string()),
                        is_wildcard: false,
                    })
                } else {
                    None
                }
            } else {
                Some(Import {
                    module: import_part.to_string(),
                    items: Vec::new(),
                    alias: None,
                    is_wildcard: false,
                })
            }
        } else if line.starts_with("from ") {
            // Parse "from module import item1, item2"
            if let Some(import_pos) = line.find(" import ") {
                let module = line[5..import_pos].trim().to_string();
                let items_part = &line[import_pos + 8..];

                if items_part.trim() == "*" {
                    Some(Import {
                        module,
                        items: Vec::new(),
                        alias: None,
                        is_wildcard: true,
                    })
                } else {
                    let items: Vec<String> = items_part
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect();

                    Some(Import {
                        module,
                        items,
                        alias: None,
                        is_wildcard: false,
                    })
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn extract_python_docstring(&self, lines: &[&str], start_line: usize) -> Option<String> {
        let line = lines[start_line].trim();
        let quote_type = if line.starts_with("\"\"\"") {
            "\"\"\""
        } else {
            "'''"
        };

        // Single line docstring
        if line.len() > 6 && line.ends_with(quote_type) {
            let content = &line[3..line.len() - 3];
            return Some(content.to_string());
        }

        // Multi-line docstring
        let mut docstring_lines = Vec::new();
        let mut current_line = start_line + 1;

        while current_line < lines.len() {
            let doc_line = lines[current_line];
            if doc_line.trim().ends_with(quote_type) {
                let final_line = doc_line.trim().trim_end_matches(quote_type);
                if !final_line.is_empty() {
                    docstring_lines.push(final_line);
                }
                break;
            }
            docstring_lines.push(doc_line.trim());
            current_line += 1;
        }

        if !docstring_lines.is_empty() {
            Some(docstring_lines.join("\n"))
        } else {
            None
        }
    }
}

/// JavaScript code parser
#[derive(Debug, Default)]
pub struct JavaScriptParser;

impl JavaScriptParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for JavaScriptParser {
    fn parse_file(&self, content: &str, _file_path: &Path) -> Result<ParseResult> {
        let result = ParseResult::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Parse imports (ES6 modules)
            if trimmed.starts_with("import ") {
                // Handle various import patterns
                continue;
            }

            // Parse require statements (CommonJS)
            if trimmed.contains("require(") {
                continue;
            }
        }

        Ok(result)
    }

    fn parse_function_signature(&self, _signature: &str) -> Result<Vec<Parameter>> {
        // JavaScript function parameter parsing
        Ok(Vec::new()) // Simplified
    }

    fn parse_type_fields(&self, _content: &str, _type_name: &str) -> Result<Vec<Field>> {
        // JavaScript class property parsing
        Ok(Vec::new()) // Simplified
    }

    fn extract_documentation(&self, _content: &str, _line_number: usize) -> Option<String> {
        // JSDoc comment extraction
        None // Simplified
    }
}

/// TypeScript code parser (extends JavaScript)
#[derive(Debug)]
pub struct TypeScriptParser {
    js_parser: JavaScriptParser,
}

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptParser {
    pub fn new() -> Self {
        Self {
            js_parser: JavaScriptParser::new(),
        }
    }
}

impl CodeParser for TypeScriptParser {
    fn parse_file(&self, content: &str, file_path: &Path) -> Result<ParseResult> {
        // Use JavaScript parser as base, then add TypeScript-specific parsing
        self.js_parser.parse_file(content, file_path)
    }

    fn parse_function_signature(&self, signature: &str) -> Result<Vec<Parameter>> {
        // TypeScript function parameter parsing with types
        self.js_parser.parse_function_signature(signature)
    }

    fn parse_type_fields(&self, content: &str, type_name: &str) -> Result<Vec<Field>> {
        // TypeScript interface/class property parsing
        self.js_parser.parse_type_fields(content, type_name)
    }

    fn extract_documentation(&self, content: &str, line_number: usize) -> Option<String> {
        self.js_parser.extract_documentation(content, line_number)
    }
}

/// Java code parser
#[derive(Debug, Default)]
pub struct JavaParser;

impl JavaParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for JavaParser {
    fn parse_file(&self, content: &str, _file_path: &Path) -> Result<ParseResult> {
        let result = ParseResult::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Parse imports
            if trimmed.starts_with("import ") {
                continue;
            }
        }

        Ok(result)
    }

    fn parse_function_signature(&self, _signature: &str) -> Result<Vec<Parameter>> {
        // Java method parameter parsing
        Ok(Vec::new()) // Simplified
    }

    fn parse_type_fields(&self, _content: &str, _type_name: &str) -> Result<Vec<Field>> {
        // Java class field parsing
        Ok(Vec::new()) // Simplified
    }

    fn extract_documentation(&self, _content: &str, _line_number: usize) -> Option<String> {
        // Javadoc comment extraction
        None // Simplified
    }
}

/// Generic parser for unsupported languages
#[derive(Debug, Default)]
pub struct GenericParser;

impl GenericParser {
    pub fn new() -> Self {
        Self
    }
}

impl CodeParser for GenericParser {
    fn parse_file(&self, _content: &str, _file_path: &Path) -> Result<ParseResult> {
        // Generic parser doesn't extract detailed syntax
        Ok(ParseResult::new())
    }

    fn parse_function_signature(&self, _signature: &str) -> Result<Vec<Parameter>> {
        Ok(Vec::new())
    }

    fn parse_type_fields(&self, _content: &str, _type_name: &str) -> Result<Vec<Field>> {
        Ok(Vec::new())
    }

    fn extract_documentation(&self, _content: &str, _line_number: usize) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_parser_function_params() {
        let parser = RustParser::new();

        let signature = "fn test_function(name: String, age: u32, active: bool = true) -> String";
        let params = parser.parse_function_signature(signature).unwrap();

        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "name");
        assert_eq!(params[0].type_annotation, "String");
        assert_eq!(params[2].default_value, Some("true".to_string()));
    }

    #[test]
    fn test_rust_parser_use_statements() {
        let parser = RustParser::new();

        let wildcard_import = parser.parse_use_statement("use std::collections::*;");
        assert!(wildcard_import.is_some());
        assert!(wildcard_import.unwrap().is_wildcard);

        let alias_import = parser.parse_use_statement("use std::collections::HashMap as Map;");
        assert!(alias_import.is_some());
        let import = alias_import.unwrap();
        assert_eq!(import.alias, Some("Map".to_string()));
    }

    #[test]
    fn test_python_parser_imports() {
        let parser = PythonParser::new();

        let simple_import = parser.parse_python_import("import os");
        assert!(simple_import.is_some());
        assert_eq!(simple_import.unwrap().module, "os");

        let from_import =
            parser.parse_python_import("from collections import defaultdict, Counter");
        assert!(from_import.is_some());
        let import = from_import.unwrap();
        assert_eq!(import.module, "collections");
        assert_eq!(import.items.len(), 2);
    }

    #[test]
    fn test_python_parser_function_params() {
        let parser = PythonParser::new();

        let signature = "def test_function(name: str, age: int = 25, *args, **kwargs) -> str:";
        let params = parser.parse_function_signature(signature).unwrap();

        assert!(!params.is_empty());
        let age_param = params.iter().find(|p| p.name == "age");
        assert!(age_param.is_some());
        assert_eq!(age_param.unwrap().default_value, Some("25".to_string()));
    }

    #[test]
    fn test_parse_result_merge() {
        let mut result1 = ParseResult::new();
        result1.functions.push(Function {
            name: "func1".to_string(),
            signature: "fn func1()".to_string(),
            documentation: None,
            parameters: Vec::new(),
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
        });

        let mut result2 = ParseResult::new();
        result2.functions.push(Function {
            name: "func2".to_string(),
            signature: "fn func2()".to_string(),
            documentation: None,
            parameters: Vec::new(),
            return_type: None,
            visibility: Visibility::Private,
            is_async: false,
        });

        result1.merge(result2);
        assert_eq!(result1.functions.len(), 2);
    }
}
