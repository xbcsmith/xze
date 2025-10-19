//! Language analyzer for repository code analysis

use crate::{
    error::{Result, XzeError},
    repository::{
        CodeStructure, ConfigFile, ConfigFormat, Field, Function, Module, Parameter,
        TypeDefinition, TypeKind, Visibility,
    },
    types::ProgrammingLanguage,
};

use std::{collections::HashMap, path::Path};

use walkdir::WalkDir;

/// Language analyzer trait for different programming languages
pub trait LanguageAnalyzer: Send + Sync {
    /// Analyze a repository and extract code structure
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure>;

    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<&'static str>;

    /// Check if this analyzer can handle the given file
    fn can_analyze(&self, file_path: &Path) -> bool {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            self.supported_extensions().contains(&ext)
        } else {
            false
        }
    }
}

/// Factory for creating language analyzers
#[derive(Debug)]
pub struct AnalyzerFactory;

impl AnalyzerFactory {
    /// Create an analyzer for the given language
    pub fn create_analyzer(language: &ProgrammingLanguage) -> Box<dyn LanguageAnalyzer> {
        match language {
            ProgrammingLanguage::Rust => Box::new(RustAnalyzer::new()),
            ProgrammingLanguage::Go => Box::new(GoAnalyzer::new()),
            ProgrammingLanguage::Python => Box::new(PythonAnalyzer::new()),
            ProgrammingLanguage::JavaScript => Box::new(JavaScriptAnalyzer::new()),
            ProgrammingLanguage::TypeScript => Box::new(TypeScriptAnalyzer::new()),
            ProgrammingLanguage::Java => Box::new(JavaAnalyzer::new()),
            _ => Box::new(GenericAnalyzer::new()),
        }
    }

    /// Auto-detect and create analyzer for a repository
    pub fn auto_detect_analyzer(
        repo_path: &Path,
    ) -> Result<(ProgrammingLanguage, Box<dyn LanguageAnalyzer>)> {
        let detected_language = Self::detect_primary_language(repo_path)?;
        let analyzer = Self::create_analyzer(&detected_language);
        Ok((detected_language, analyzer))
    }

    /// Detect the primary programming language in a repository
    fn detect_primary_language(repo_path: &Path) -> Result<ProgrammingLanguage> {
        let mut language_counts: HashMap<ProgrammingLanguage, usize> = HashMap::new();

        for entry in WalkDir::new(repo_path).max_depth(5) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                    let lang = ProgrammingLanguage::from(extension);
                    if !matches!(lang, ProgrammingLanguage::Unknown(_)) {
                        *language_counts.entry(lang).or_insert(0) += 1;
                    }
                }

                // Check for specific files that indicate language
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    match filename {
                        "Cargo.toml" | "Cargo.lock" => {
                            *language_counts
                                .entry(ProgrammingLanguage::Rust)
                                .or_insert(0) += 10;
                        }
                        "go.mod" | "go.sum" => {
                            *language_counts.entry(ProgrammingLanguage::Go).or_insert(0) += 10;
                        }
                        "package.json" | "package-lock.json" => {
                            *language_counts
                                .entry(ProgrammingLanguage::JavaScript)
                                .or_insert(0) += 10;
                        }
                        "requirements.txt" | "pyproject.toml" | "setup.py" => {
                            *language_counts
                                .entry(ProgrammingLanguage::Python)
                                .or_insert(0) += 10;
                        }
                        "pom.xml" | "build.gradle" => {
                            *language_counts
                                .entry(ProgrammingLanguage::Java)
                                .or_insert(0) += 10;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Return the most common language
        Ok(language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
            .unwrap_or(ProgrammingLanguage::Unknown("mixed".to_string())))
    }
}

/// Rust language analyzer
#[derive(Debug, Default)]
pub struct RustAnalyzer;

impl RustAnalyzer {
    pub fn new() -> Self {
        Self
    }

    fn extract_rust_doc_comment(content: &str, line_start: usize) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut doc_lines = Vec::new();
        let mut current_line = line_start;

        // Look backwards for doc comments
        while current_line > 0 {
            current_line -= 1;
            let line = lines.get(current_line)?.trim();
            if line.starts_with("///") {
                doc_lines.insert(0, line.trim_start_matches("///").trim());
            } else if line.starts_with("//!") {
                doc_lines.insert(0, line.trim_start_matches("//!").trim());
            } else if line.is_empty() {
                continue;
            } else {
                break;
            }
        }

        if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        }
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure> {
        let mut structure = CodeStructure::new();

        // Find all Rust files
        for entry in WalkDir::new(repo_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    self.parse_rust_file(path, &content, &mut structure)?;
                }
            }
        }

        // Look for Cargo.toml and other config files
        self.parse_cargo_files(repo_path, &mut structure)?;

        Ok(structure)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["rs"]
    }
}

impl RustAnalyzer {
    fn parse_rust_file(
        &self,
        file_path: &Path,
        content: &str,
        structure: &mut CodeStructure,
    ) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Parse modules
            if let Some(module_name) = self.extract_module_name(trimmed) {
                let visibility = if trimmed.starts_with("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                };

                structure.modules.push(Module {
                    name: module_name,
                    path: file_path.to_path_buf(),
                    documentation: Self::extract_rust_doc_comment(content, line_num),
                    visibility,
                });
            }

            // Parse functions
            if let Some(function) = self.extract_function(trimmed, content, line_num) {
                structure.functions.push(function);
            }

            // Parse structs and enums
            if let Some(type_def) = self.extract_type_definition(trimmed, content, line_num) {
                structure.types.push(type_def);
            }
        }

        Ok(())
    }

    fn extract_module_name(&self, line: &str) -> Option<String> {
        if line.starts_with("mod ") || line.starts_with("pub mod ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[parts.len() - 1].trim_end_matches([';', '{']);
                return Some(name.to_string());
            }
        }
        None
    }

    fn extract_function(&self, line: &str, content: &str, line_num: usize) -> Option<Function> {
        if line.contains("fn ") && !line.trim_start().starts_with("//") {
            let visibility = if line.contains("pub fn") {
                Visibility::Public
            } else {
                Visibility::Private
            };

            let is_async = line.contains("async fn");

            // Extract function name
            let fn_start = line.find("fn ")?;
            let after_fn = &line[fn_start + 3..];
            let name_end = after_fn.find('(')?;
            let name = after_fn[..name_end].trim().to_string();

            // Extract full signature (may span multiple lines)
            let signature = self.extract_full_signature(content, line_num);

            // Parse parameters from signature
            let parameters = self.parse_function_parameters(&signature);

            // Parse return type
            let return_type = self.parse_return_type(&signature);

            Some(Function {
                name,
                signature: signature.trim().to_string(),
                documentation: Self::extract_rust_doc_comment(content, line_num),
                parameters,
                return_type,
                visibility,
                is_async,
            })
        } else {
            None
        }
    }

    fn extract_full_signature(&self, content: &str, start_line: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut signature = String::new();
        let mut paren_count = 0;

        for line in lines.iter().skip(start_line) {
            let trimmed = line.trim();

            paren_count += trimmed.matches('(').count() as i32;
            paren_count -= trimmed.matches(')').count() as i32;

            signature.push_str(trimmed);
            signature.push(' ');

            // Stop at opening brace or semicolon if parentheses are balanced
            if paren_count == 0 && (trimmed.contains('{') || trimmed.ends_with(';')) {
                break;
            }
        }

        signature
    }

    fn parse_function_parameters(&self, signature: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        // Find parameter list between parentheses
        let start = match signature.find('(') {
            Some(pos) => pos,
            None => return params,
        };

        let end = match signature.rfind(')') {
            Some(pos) => pos,
            None => return params,
        };

        if start >= end {
            return params;
        }

        let param_str = &signature[start + 1..end];
        if param_str.trim().is_empty() {
            return params;
        }

        // Split by comma, respecting nested generics and parentheses
        let param_parts = self.split_parameters(param_str);

        for part in param_parts {
            let part = part.trim();
            if part.is_empty() || part == "&self" || part == "&mut self" || part == "self" {
                continue;
            }

            // Remove 'mut' keyword if present
            let mut param_str = part;
            if param_str.starts_with("mut ") {
                param_str = &param_str[4..];
            }

            // Parse pattern: name: Type
            if let Some(colon_pos) = param_str.find(':') {
                let name = param_str[..colon_pos].trim().to_string();
                let type_annotation = param_str[colon_pos + 1..].trim().to_string();

                params.push(Parameter {
                    name,
                    type_annotation,
                    default_value: None,
                });
            }
        }

        params
    }

    fn split_parameters(&self, params_str: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut angle_depth = 0;
        let mut paren_depth = 0;

        for ch in params_str.chars() {
            match ch {
                '<' => {
                    angle_depth += 1;
                    current.push(ch);
                }
                '>' => {
                    angle_depth -= 1;
                    current.push(ch);
                }
                '(' => {
                    paren_depth += 1;
                    current.push(ch);
                }
                ')' => {
                    paren_depth -= 1;
                    current.push(ch);
                }
                ',' if angle_depth == 0 && paren_depth == 0 => {
                    if !current.trim().is_empty() {
                        result.push(current.trim().to_string());
                        current.clear();
                    }
                }
                _ => current.push(ch),
            }
        }

        if !current.trim().is_empty() {
            result.push(current.trim().to_string());
        }

        result
    }

    fn parse_return_type(&self, signature: &str) -> Option<String> {
        // Find return type after ->
        if let Some(arrow_pos) = signature.find("->") {
            let after_arrow = &signature[arrow_pos + 2..];

            // Find the end of the return type (before where/{ or end of string)
            let end_pos = after_arrow
                .find("where")
                .or_else(|| after_arrow.find('{'))
                .or_else(|| after_arrow.find(';'))
                .unwrap_or(after_arrow.len());

            let return_type = after_arrow[..end_pos].trim();
            if !return_type.is_empty() {
                return Some(return_type.to_string());
            }
        }
        None
    }

    fn extract_type_definition(
        &self,
        line: &str,
        content: &str,
        line_num: usize,
    ) -> Option<TypeDefinition> {
        let trimmed = line.trim();

        if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
            return self.extract_struct(trimmed, content, line_num);
        }

        if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
            return self.extract_enum(trimmed, content, line_num);
        }

        if trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") {
            return self.extract_trait(trimmed, content, line_num);
        }

        None
    }

    fn extract_struct(&self, line: &str, content: &str, line_num: usize) -> Option<TypeDefinition> {
        let visibility = if line.starts_with("pub") {
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Extract struct name
        let parts: Vec<&str> = line.split_whitespace().collect();
        let name = parts.get(1)?.trim_end_matches(['{', ';']).to_string();

        // Parse struct fields
        let fields = self.parse_struct_fields(content, line_num);

        Some(TypeDefinition {
            name,
            kind: TypeKind::Struct,
            documentation: Self::extract_rust_doc_comment(content, line_num),
            fields,
            visibility,
        })
    }

    fn extract_enum(&self, line: &str, content: &str, line_num: usize) -> Option<TypeDefinition> {
        let visibility = if line.starts_with("pub") {
            Visibility::Public
        } else {
            Visibility::Private
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        let name = parts.get(1)?.trim_end_matches(['{', ';']).to_string();

        // Parse enum variants
        let fields = self.parse_enum_variants(content, line_num);

        Some(TypeDefinition {
            name,
            kind: TypeKind::Enum,
            documentation: Self::extract_rust_doc_comment(content, line_num),
            fields,
            visibility,
        })
    }

    fn extract_trait(&self, line: &str, content: &str, line_num: usize) -> Option<TypeDefinition> {
        let visibility = if line.starts_with("pub") {
            Visibility::Public
        } else {
            Visibility::Private
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        let name = parts.get(1)?.trim_end_matches(['{', ';']).to_string();

        Some(TypeDefinition {
            name,
            kind: TypeKind::Trait,
            documentation: Self::extract_rust_doc_comment(content, line_num),
            fields: Vec::new(),
            visibility,
        })
    }

    fn parse_struct_fields(&self, content: &str, start_line: usize) -> Vec<Field> {
        let lines: Vec<&str> = content.lines().collect();
        let mut fields = Vec::new();

        if start_line >= lines.len() {
            return fields;
        }

        let mut in_struct_body = false;
        let mut brace_count = 0;
        let mut current_doc = None;

        for (_idx, line) in lines.iter().enumerate().skip(start_line) {
            let trimmed = line.trim();

            // Track documentation comments
            if trimmed.starts_with("///") {
                let doc = trimmed.trim_start_matches("///").trim();
                current_doc = Some(match current_doc {
                    Some(existing) => format!("{}\n{}", existing, doc),
                    None => doc.to_string(),
                });
                continue;
            }

            // Find struct body
            if trimmed.contains('{') {
                in_struct_body = true;
                brace_count += trimmed.matches('{').count();
            }

            if trimmed.contains('}') {
                brace_count -= trimmed.matches('}').count();
                if brace_count == 0 {
                    break;
                }
            }

            if !in_struct_body {
                continue;
            }

            // Parse field: pub name: Type,
            if trimmed.contains(':') && !trimmed.starts_with("//") {
                let field_line = trimmed.trim_end_matches(',');

                // Remove visibility modifiers
                let field_line = field_line
                    .trim_start_matches("pub ")
                    .trim_start_matches("pub(crate) ")
                    .trim_start_matches("pub(super) ");

                if let Some(colon_pos) = field_line.find(':') {
                    let name = field_line[..colon_pos].trim().to_string();
                    let type_annotation = field_line[colon_pos + 1..].trim().to_string();

                    if !name.is_empty() && !type_annotation.is_empty() {
                        fields.push(Field {
                            name,
                            type_annotation,
                            documentation: current_doc.take(),
                        });
                    }
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
                // Clear doc comment if we hit a non-field line
                current_doc = None;
            }
        }

        fields
    }

    fn parse_enum_variants(&self, content: &str, start_line: usize) -> Vec<Field> {
        let lines: Vec<&str> = content.lines().collect();
        let mut variants = Vec::new();

        if start_line >= lines.len() {
            return variants;
        }

        let mut in_enum_body = false;
        let mut brace_count = 0;
        let mut current_doc = None;

        for line in lines.iter().skip(start_line) {
            let trimmed = line.trim();

            // Track documentation comments
            if trimmed.starts_with("///") {
                let doc = trimmed.trim_start_matches("///").trim();
                current_doc = Some(match current_doc {
                    Some(existing) => format!("{}\n{}", existing, doc),
                    None => doc.to_string(),
                });
                continue;
            }

            if trimmed.contains('{') {
                in_enum_body = true;
                brace_count += trimmed.matches('{').count();
            }

            if trimmed.contains('}') {
                brace_count -= trimmed.matches('}').count();
                if brace_count == 0 {
                    break;
                }
            }

            if !in_enum_body || trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            // Parse variant: VariantName or VariantName(Type) or VariantName { fields }
            let variant_line = trimmed.trim_end_matches(',');

            let variant_name = if let Some(paren_pos) = variant_line.find('(') {
                variant_line[..paren_pos].trim()
            } else if let Some(brace_pos) = variant_line.find('{') {
                variant_line[..brace_pos].trim()
            } else {
                variant_line.trim()
            };

            if !variant_name.is_empty() && variant_name.chars().next().unwrap().is_uppercase() {
                variants.push(Field {
                    name: variant_name.to_string(),
                    type_annotation: "variant".to_string(),
                    documentation: current_doc.take(),
                });
            }
        }

        variants
    }

    fn parse_cargo_files(&self, repo_path: &Path, structure: &mut CodeStructure) -> Result<()> {
        let cargo_toml = repo_path.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                structure.configs.push(ConfigFile {
                    path: cargo_toml,
                    format: ConfigFormat::Toml,
                    content,
                });
            }
        }

        let cargo_lock = repo_path.join("Cargo.lock");
        if cargo_lock.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_lock) {
                structure.configs.push(ConfigFile {
                    path: cargo_lock,
                    format: ConfigFormat::Toml,
                    content,
                });
            }
        }

        Ok(())
    }
}

/// Go language analyzer
#[derive(Debug, Default)]
pub struct GoAnalyzer;

impl GoAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAnalyzer for GoAnalyzer {
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure> {
        let mut structure = CodeStructure::new();

        for entry in WalkDir::new(repo_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("go") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    self.parse_go_file(path, &content, &mut structure)?;
                }
            }
        }

        self.parse_go_mod(repo_path, &mut structure)?;
        Ok(structure)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["go"]
    }
}

impl GoAnalyzer {
    fn parse_go_file(
        &self,
        _file_path: &Path,
        content: &str,
        structure: &mut CodeStructure,
    ) -> Result<()> {
        // Simple Go parsing - extract functions, types, etc.
        for line in content.lines() {
            let trimmed = line.trim();

            // Parse functions
            if trimmed.starts_with("func ") {
                if let Some(function) = self.extract_go_function(trimmed) {
                    structure.functions.push(function);
                }
            }

            // Parse types (structs, interfaces)
            if trimmed.starts_with("type ") {
                if let Some(type_def) = self.extract_go_type(trimmed) {
                    structure.types.push(type_def);
                }
            }
        }

        Ok(())
    }

    fn extract_go_function(&self, line: &str) -> Option<Function> {
        // Extract function name from "func functionName(...) ..."
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name_with_params = parts[1];
            let name = if let Some(paren_pos) = name_with_params.find('(') {
                name_with_params[..paren_pos].to_string()
            } else {
                name_with_params.to_string()
            };

            // Go functions are public if they start with uppercase
            let visibility = if name.chars().next().unwrap_or('a').is_uppercase() {
                Visibility::Public
            } else {
                Visibility::Private
            };

            Some(Function {
                name,
                signature: line.to_string(),
                documentation: None, // TODO: Extract Go doc comments
                parameters: Vec::new(),
                return_type: None,
                visibility,
                is_async: false, // Go doesn't have async functions in the same way
            })
        } else {
            None
        }
    }

    fn extract_go_type(&self, line: &str) -> Option<TypeDefinition> {
        // Parse "type TypeName struct/interface/..."
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let name = parts[1].to_string();
            let type_keyword = parts[2];

            let kind = match type_keyword {
                "struct" => TypeKind::Struct,
                "interface" => TypeKind::Interface,
                _ => return None,
            };

            let visibility = if name.chars().next().unwrap_or('a').is_uppercase() {
                Visibility::Public
            } else {
                Visibility::Private
            };

            Some(TypeDefinition {
                name,
                kind,
                documentation: None,
                fields: Vec::new(),
                visibility,
            })
        } else {
            None
        }
    }

    fn parse_go_mod(&self, repo_path: &Path, structure: &mut CodeStructure) -> Result<()> {
        let go_mod = repo_path.join("go.mod");
        if go_mod.exists() {
            if let Ok(content) = std::fs::read_to_string(&go_mod) {
                structure.configs.push(ConfigFile {
                    path: go_mod,
                    format: ConfigFormat::Toml, // go.mod is similar to TOML
                    content,
                });
            }
        }
        Ok(())
    }
}

/// Python language analyzer
#[derive(Debug, Default)]
pub struct PythonAnalyzer;

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure> {
        let mut structure = CodeStructure::new();

        for entry in WalkDir::new(repo_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("py") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    self.parse_python_file(path, &content, &mut structure)?;
                }
            }
        }

        self.parse_python_configs(repo_path, &mut structure)?;
        Ok(structure)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["py"]
    }
}

impl PythonAnalyzer {
    fn parse_python_file(
        &self,
        _file_path: &Path,
        content: &str,
        structure: &mut CodeStructure,
    ) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Parse functions
            if line.starts_with("def ") {
                if let Some(function) = self.extract_python_function(line, &lines, i) {
                    structure.functions.push(function);
                }
            }

            // Parse classes
            if line.starts_with("class ") {
                if let Some(class_def) = self.extract_python_class(line, &lines, i) {
                    structure.types.push(class_def);
                }
            }

            i += 1;
        }

        Ok(())
    }

    fn extract_python_function(
        &self,
        line: &str,
        lines: &[&str],
        line_index: usize,
    ) -> Option<Function> {
        // Extract function name from "def function_name(...):"
        let def_start = line.find("def ")?;
        let after_def = &line[def_start + 4..];
        let name_end = after_def.find('(')?;
        let name = after_def[..name_end].trim().to_string();

        // Python uses naming convention for visibility
        let visibility = if name.starts_with('_') {
            Visibility::Private
        } else {
            Visibility::Public
        };

        // Check if it's async
        let is_async = line.contains("async def");

        // Extract docstring
        let documentation = self.extract_python_docstring(lines, line_index + 1);

        Some(Function {
            name,
            signature: line.to_string(),
            documentation,
            parameters: Vec::new(), // TODO: Parse parameters
            return_type: None,      // TODO: Parse type annotations
            visibility,
            is_async,
        })
    }

    fn extract_python_class(
        &self,
        line: &str,
        lines: &[&str],
        line_index: usize,
    ) -> Option<TypeDefinition> {
        // Extract class name from "class ClassName(...):"
        let class_start = line.find("class ")?;
        let after_class = &line[class_start + 6..];
        let name_end = after_class.find(['(', ':']).unwrap_or(after_class.len());
        let name = after_class[..name_end].trim().to_string();

        let visibility = if name.starts_with('_') {
            Visibility::Private
        } else {
            Visibility::Public
        };

        let documentation = self.extract_python_docstring(lines, line_index + 1);

        Some(TypeDefinition {
            name,
            kind: TypeKind::Class,
            documentation,
            fields: Vec::new(), // TODO: Parse class attributes
            visibility,
        })
    }

    fn extract_python_docstring(&self, lines: &[&str], start_index: usize) -> Option<String> {
        if start_index >= lines.len() {
            return None;
        }

        let mut current_line = start_index;

        // Skip empty lines
        while current_line < lines.len() && lines[current_line].trim().is_empty() {
            current_line += 1;
        }

        if current_line >= lines.len() {
            return None;
        }

        let line = lines[current_line].trim();

        // Check for docstring
        if line.starts_with("\"\"\"") || line.starts_with("'''") {
            let quote_type = if line.starts_with("\"\"\"") {
                "\"\"\""
            } else {
                "'''"
            };
            let mut docstring_lines = Vec::new();

            // Single line docstring
            if line.len() > 6 && line.ends_with(quote_type) {
                let content = &line[3..line.len() - 3];
                return Some(content.to_string());
            }

            // Multi-line docstring
            current_line += 1;
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
                return Some(docstring_lines.join("\n"));
            }
        }

        None
    }

    fn parse_python_configs(&self, repo_path: &Path, structure: &mut CodeStructure) -> Result<()> {
        // Parse requirements.txt
        let requirements = repo_path.join("requirements.txt");
        if requirements.exists() {
            if let Ok(content) = std::fs::read_to_string(&requirements) {
                structure.configs.push(ConfigFile {
                    path: requirements,
                    format: ConfigFormat::Env, // Plain text format
                    content,
                });
            }
        }

        // Parse pyproject.toml
        let pyproject = repo_path.join("pyproject.toml");
        if pyproject.exists() {
            if let Ok(content) = std::fs::read_to_string(&pyproject) {
                structure.configs.push(ConfigFile {
                    path: pyproject,
                    format: ConfigFormat::Toml,
                    content,
                });
            }
        }

        Ok(())
    }
}

/// JavaScript/TypeScript analyzer
#[derive(Debug, Default)]
pub struct JavaScriptAnalyzer;

impl JavaScriptAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAnalyzer for JavaScriptAnalyzer {
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure> {
        let mut structure = CodeStructure::new();

        for entry in WalkDir::new(repo_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "js" | "mjs" | "cjs") {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        self.parse_js_file(path, &content, &mut structure)?;
                    }
                }
            }
        }

        self.parse_js_configs(repo_path, &mut structure)?;
        Ok(structure)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["js", "mjs", "cjs"]
    }
}

impl JavaScriptAnalyzer {
    fn parse_js_file(
        &self,
        _file_path: &Path,
        content: &str,
        structure: &mut CodeStructure,
    ) -> Result<()> {
        // Simple JavaScript parsing
        for line in content.lines() {
            let trimmed = line.trim();

            // Parse functions
            if trimmed.starts_with("function ")
                || trimmed.contains("= function")
                || trimmed.contains("=> ")
            {
                if let Some(function) = self.extract_js_function(trimmed) {
                    structure.functions.push(function);
                }
            }

            // Parse classes
            if trimmed.starts_with("class ") {
                if let Some(class_def) = self.extract_js_class(trimmed) {
                    structure.types.push(class_def);
                }
            }
        }

        Ok(())
    }

    fn extract_js_function(&self, line: &str) -> Option<Function> {
        let name = if let Some(after_func) = line.strip_prefix("function ") {
            // function functionName(...)
            let name_end = after_func.find('(').unwrap_or(after_func.len());
            after_func[..name_end].trim().to_string()
        } else if let Some(eq_pos) = line.find('=') {
            // const functionName = function... or const functionName = (...) =>
            let before_eq = &line[..eq_pos];
            let parts: Vec<&str> = before_eq.split_whitespace().collect();
            parts.last()?.to_string()
        } else {
            return None;
        };

        let is_async = line.contains("async");

        Some(Function {
            name,
            signature: line.to_string(),
            documentation: None, // TODO: Extract JSDoc
            parameters: Vec::new(),
            return_type: None,
            visibility: Visibility::Public, // JavaScript doesn't have private functions in the same way
            is_async,
        })
    }

    fn extract_js_class(&self, line: &str) -> Option<TypeDefinition> {
        let class_start = line.find("class ")?;
        let after_class = &line[class_start + 6..];
        let name_end = after_class
            .find([' ', '{', 'e'])
            .unwrap_or(after_class.len()); // 'e' for 'extends'
        let name = after_class[..name_end].trim().to_string();

        Some(TypeDefinition {
            name,
            kind: TypeKind::Class,
            documentation: None,
            fields: Vec::new(),
            visibility: Visibility::Public,
        })
    }

    fn parse_js_configs(&self, repo_path: &Path, structure: &mut CodeStructure) -> Result<()> {
        let package_json = repo_path.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                structure.configs.push(ConfigFile {
                    path: package_json,
                    format: ConfigFormat::Json,
                    content,
                });
            }
        }
        Ok(())
    }
}

/// TypeScript analyzer (extends JavaScript)
#[derive(Debug)]
pub struct TypeScriptAnalyzer {
    js_analyzer: JavaScriptAnalyzer,
}

impl Default for TypeScriptAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptAnalyzer {
    pub fn new() -> Self {
        Self {
            js_analyzer: JavaScriptAnalyzer::new(),
        }
    }
}

impl LanguageAnalyzer for TypeScriptAnalyzer {
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure> {
        let mut structure = self.js_analyzer.analyze(repo_path)?;

        // Also parse TypeScript files
        for entry in WalkDir::new(repo_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "ts" | "tsx") {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        self.parse_ts_file(path, &content, &mut structure)?;
                    }
                }
            }
        }

        Ok(structure)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["js", "mjs", "cjs", "ts", "tsx"]
    }
}

impl TypeScriptAnalyzer {
    fn parse_ts_file(
        &self,
        file_path: &Path,
        content: &str,
        structure: &mut CodeStructure,
    ) -> Result<()> {
        // Parse TypeScript-specific constructs
        for line in content.lines() {
            let trimmed = line.trim();

            // Parse interfaces
            if trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") {
                if let Some(interface_def) = self.extract_ts_interface(trimmed) {
                    structure.types.push(interface_def);
                }
            }

            // Parse type aliases
            if trimmed.starts_with("type ") || trimmed.starts_with("export type ") {
                if let Some(type_def) = self.extract_ts_type_alias(trimmed) {
                    structure.types.push(type_def);
                }
            }
        }

        // Also parse as JavaScript
        self.js_analyzer
            .parse_js_file(file_path, content, structure)?;
        Ok(())
    }

    fn extract_ts_interface(&self, line: &str) -> Option<TypeDefinition> {
        let interface_start = line.find("interface ")?;
        let after_interface = &line[interface_start + 10..];
        let name_end = after_interface
            .find([' ', '{', '<'])
            .unwrap_or(after_interface.len());
        let name = after_interface[..name_end].trim().to_string();

        Some(TypeDefinition {
            name,
            kind: TypeKind::Interface,
            documentation: None,
            fields: Vec::new(),
            visibility: Visibility::Public,
        })
    }

    fn extract_ts_type_alias(&self, line: &str) -> Option<TypeDefinition> {
        let type_start = line.find("type ")?;
        let after_type = &line[type_start + 5..];
        let name_end = after_type.find([' ', '=', '<']).unwrap_or(after_type.len());
        let name = after_type[..name_end].trim().to_string();

        Some(TypeDefinition {
            name,
            kind: TypeKind::Interface, // Type aliases are similar to interfaces
            documentation: None,
            fields: Vec::new(),
            visibility: Visibility::Public,
        })
    }
}

/// Java language analyzer
#[derive(Debug, Default)]
pub struct JavaAnalyzer;

impl JavaAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAnalyzer for JavaAnalyzer {
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure> {
        let mut structure = CodeStructure::new();

        for entry in WalkDir::new(repo_path) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("java") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    self.parse_java_file(path, &content, &mut structure)?;
                }
            }
        }

        Ok(structure)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["java"]
    }
}

impl JavaAnalyzer {
    fn parse_java_file(
        &self,
        _file_path: &Path,
        content: &str,
        structure: &mut CodeStructure,
    ) -> Result<()> {
        for line in content.lines() {
            let trimmed = line.trim();

            // Parse methods
            if self.is_java_method(trimmed) {
                if let Some(method) = self.extract_java_method(trimmed) {
                    structure.functions.push(method);
                }
            }

            // Parse classes/interfaces
            if trimmed.contains("class ") || trimmed.contains("interface ") {
                if let Some(type_def) = self.extract_java_type(trimmed) {
                    structure.types.push(type_def);
                }
            }
        }

        Ok(())
    }

    fn is_java_method(&self, line: &str) -> bool {
        // Simple heuristic: contains visibility modifier and parentheses
        (line.contains("public ") || line.contains("private ") || line.contains("protected "))
            && line.contains('(')
            && line.contains(')')
            && !line.contains("class ")
            && !line.contains("interface ")
    }

    fn extract_java_method(&self, line: &str) -> Option<Function> {
        // Extract method name (simplified)
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut name = String::new();

        for part in parts.iter() {
            if part.contains('(') {
                let method_part = part.split('(').next()?;
                name = method_part.to_string();
                break;
            }
        }

        if name.is_empty() {
            return None;
        }

        let visibility = if line.contains("public ") {
            Visibility::Public
        } else if line.contains("protected ") {
            Visibility::Protected
        } else {
            Visibility::Private
        };

        Some(Function {
            name,
            signature: line.to_string(),
            documentation: None, // TODO: Extract Javadoc
            parameters: Vec::new(),
            return_type: None,
            visibility,
            is_async: false,
        })
    }

    fn extract_java_type(&self, line: &str) -> Option<TypeDefinition> {
        let is_class = line.contains("class ");
        let is_interface = line.contains("interface ");

        if !is_class && !is_interface {
            return None;
        }

        let keyword = if is_class { "class " } else { "interface " };
        let start_pos = line.find(keyword)?;
        let after_keyword = &line[start_pos + keyword.len()..];
        let name_end = after_keyword
            .find([' ', '{', '<'])
            .unwrap_or(after_keyword.len());
        let name = after_keyword[..name_end].trim().to_string();

        let visibility = if line.contains("public ") {
            Visibility::Public
        } else if line.contains("protected ") {
            Visibility::Protected
        } else {
            Visibility::Private
        };

        let kind = if is_class {
            TypeKind::Class
        } else {
            TypeKind::Interface
        };

        Some(TypeDefinition {
            name,
            kind,
            documentation: None,
            fields: Vec::new(),
            visibility,
        })
    }
}

/// Generic analyzer for unsupported languages
#[derive(Debug, Default)]
pub struct GenericAnalyzer;

impl GenericAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAnalyzer for GenericAnalyzer {
    fn analyze(&self, repo_path: &Path) -> Result<CodeStructure> {
        let mut structure = CodeStructure::new();

        // Only parse configuration files
        for entry in WalkDir::new(repo_path).max_depth(2) {
            let entry = entry.map_err(|e| XzeError::filesystem(format!("Walk error: {}", e)))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(config_file) = self.try_parse_config_file(path)? {
                    structure.configs.push(config_file);
                }
            }
        }

        Ok(structure)
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["*"] // Generic analyzer supports all files
    }
}

impl GenericAnalyzer {
    fn try_parse_config_file(&self, path: &Path) -> Result<Option<ConfigFile>> {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            if let Some(format) = ConfigFormat::from_extension(extension) {
                if let Ok(content) = std::fs::read_to_string(path) {
                    return Ok(Some(ConfigFile {
                        path: path.to_path_buf(),
                        format,
                        content,
                    }));
                }
            }
        }

        // Check for specific config files without extensions
        if let Some("Dockerfile" | "Makefile" | "README" | "LICENSE") =
            path.file_name().and_then(|n| n.to_str())
        {
            if let Ok(content) = std::fs::read_to_string(path) {
                return Ok(Some(ConfigFile {
                    path: path.to_path_buf(),
                    format: ConfigFormat::Env,
                    content,
                }));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_analyzer_factory() {
        let rust_analyzer = AnalyzerFactory::create_analyzer(&ProgrammingLanguage::Rust);
        assert_eq!(rust_analyzer.supported_extensions(), vec!["rs"]);

        let go_analyzer = AnalyzerFactory::create_analyzer(&ProgrammingLanguage::Go);
        assert_eq!(go_analyzer.supported_extensions(), vec!["go"]);
    }

    #[test]
    fn test_rust_analyzer() {
        let temp_dir = TempDir::new().unwrap();

        // Create a simple Rust file
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let rust_file = src_dir.join("main.rs");
        fs::write(
            &rust_file,
            r#"
/// Main function
pub fn main() {
    println!("Hello, world!");
}

/// A test struct
pub struct TestStruct {
    pub field: String,
}

impl TestStruct {
    /// Create a new instance
    pub fn new(field: String) -> Self {
        Self { field }
    }
}
"#,
        )
        .unwrap();

        let analyzer = RustAnalyzer::new();
        let structure = analyzer.analyze(temp_dir.path()).unwrap();

        assert!(!structure.functions.is_empty());
        assert!(!structure.types.is_empty());

        // Check that main function is found
        let main_fn = structure.functions.iter().find(|f| f.name == "main");
        assert!(main_fn.is_some());
        assert_eq!(main_fn.unwrap().visibility, Visibility::Public);
    }

    #[test]
    fn test_python_analyzer() {
        let temp_dir = TempDir::new().unwrap();

        let python_file = temp_dir.path().join("main.py");
        fs::write(
            &python_file,
            r#"
def hello_world():
    """Print hello world message."""
    print("Hello, world!")

class TestClass:
    """A test class."""

    def __init__(self):
        """Initialize the class."""
        pass

    def _private_method(self):
        """A private method."""
        pass
"#,
        )
        .unwrap();

        let analyzer = PythonAnalyzer::new();
        let structure = analyzer.analyze(temp_dir.path()).unwrap();

        assert!(!structure.functions.is_empty());
        assert!(!structure.types.is_empty());

        // Check function visibility
        let hello_fn = structure.functions.iter().find(|f| f.name == "hello_world");
        assert!(hello_fn.is_some());
        assert_eq!(hello_fn.unwrap().visibility, Visibility::Public);

        let private_fn = structure
            .functions
            .iter()
            .find(|f| f.name == "_private_method");
        assert!(private_fn.is_some());
        assert_eq!(private_fn.unwrap().visibility, Visibility::Private);
    }

    #[test]
    fn test_language_detection() {
        let temp_dir = TempDir::new().unwrap();

        // Create files for different languages
        fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(temp_dir.path().join("script.py"), "print('hello')").unwrap();

        let (detected_lang, _) = AnalyzerFactory::auto_detect_analyzer(temp_dir.path()).unwrap();

        // Should detect Rust due to Cargo.toml having higher weight
        assert_eq!(detected_lang, ProgrammingLanguage::Rust);
    }

    #[test]
    fn test_generic_analyzer() {
        let temp_dir = TempDir::new().unwrap();

        // Create some config files
        fs::write(temp_dir.path().join("config.yaml"), "key: value").unwrap();
        fs::write(temp_dir.path().join("data.json"), r#"{"key": "value"}"#).unwrap();
        fs::write(temp_dir.path().join("Dockerfile"), "FROM ubuntu").unwrap();

        let analyzer = GenericAnalyzer::new();
        let structure = analyzer.analyze(temp_dir.path()).unwrap();

        assert_eq!(structure.configs.len(), 3);

        let yaml_config = structure
            .configs
            .iter()
            .find(|c| c.path.ends_with("config.yaml"));
        assert!(yaml_config.is_some());
        assert_eq!(yaml_config.unwrap().format, ConfigFormat::Yaml);
    }
}
