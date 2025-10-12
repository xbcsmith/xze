//! AI prompt templates for documentation generation

use crate::{repository::CodeStructure, types::DiátaxisCategory};
use handlebars::Handlebars;
use serde_json::json;
use std::collections::HashMap;

/// Prompt template library for AI documentation generation
#[derive(Debug)]
pub struct PromptTemplateLibrary {
    handlebars: Handlebars<'static>,
    templates: HashMap<String, String>,
}

impl PromptTemplateLibrary {
    /// Create a new prompt template library
    pub fn new() -> Self {
        let mut library = Self {
            handlebars: Handlebars::new(),
            templates: HashMap::new(),
        };

        library.register_default_templates();
        library
    }

    /// Register all default templates
    fn register_default_templates(&mut self) {
        // Code analysis template
        self.register_template(
            "code_analysis",
            include_str!("../../templates/code_analysis.hbs"),
        );

        // Documentation templates for each Diátaxis category
        self.register_template("tutorial", include_str!("../../templates/tutorial.hbs"));
        self.register_template("howto", include_str!("../../templates/howto.hbs"));
        self.register_template("reference", include_str!("../../templates/reference.hbs"));
        self.register_template(
            "explanation",
            include_str!("../../templates/explanation.hbs"),
        );

        // API documentation template
        self.register_template("api_docs", include_str!("../../templates/api_docs.hbs"));

        // Summary templates
        self.register_template("summary", include_str!("../../templates/summary.hbs"));
    }

    /// Register a template with fallback to built-in template if file doesn't exist
    fn register_template(&mut self, name: &str, template_str: &str) {
        let template = if template_str.is_empty() {
            // Fallback to built-in templates if files don't exist
            self.get_builtin_template(name)
        } else {
            template_str.to_string()
        };

        self.templates.insert(name.to_string(), template.clone());

        if let Err(e) = self.handlebars.register_template_string(name, &template) {
            tracing::warn!("Failed to register template '{}': {}", name, e);
            // Register a simple fallback template
            let fallback = format!(
                "Error loading template '{}'. Please check template syntax.",
                name
            );
            let _ = self.handlebars.register_template_string(name, &fallback);
        }
    }

    /// Get built-in template content
    fn get_builtin_template(&self, name: &str) -> String {
        match name {
            "code_analysis" => BUILTIN_CODE_ANALYSIS_TEMPLATE.to_string(),
            "tutorial" => BUILTIN_TUTORIAL_TEMPLATE.to_string(),
            "howto" => BUILTIN_HOWTO_TEMPLATE.to_string(),
            "reference" => BUILTIN_REFERENCE_TEMPLATE.to_string(),
            "explanation" => BUILTIN_EXPLANATION_TEMPLATE.to_string(),
            "api_docs" => BUILTIN_API_DOCS_TEMPLATE.to_string(),
            "summary" => BUILTIN_SUMMARY_TEMPLATE.to_string(),
            _ => format!("Unknown template: {}", name),
        }
    }

    /// Generate code analysis prompt
    pub fn code_analysis_prompt(&self, structure: &CodeStructure) -> String {
        let data = json!({
            "total_items": structure.item_count(),
            "modules": structure.modules,
            "functions": structure.functions,
            "types": structure.types,
            "configs": structure.configs,
            "public_functions": structure.public_functions().len(),
        });

        self.render_template("code_analysis", &data)
            .unwrap_or_else(|_| {
                format!(
                    "Analyze this codebase with {} items",
                    structure.item_count()
                )
            })
    }

    /// Generate API documentation prompt
    pub fn api_documentation_prompt(&self, structure: &CodeStructure) -> String {
        let public_functions = structure.public_functions();
        let data = json!({
            "functions": public_functions,
            "types": structure.types.iter().filter(|t| t.visibility == crate::repository::Visibility::Public).collect::<Vec<_>>(),
            "modules": structure.modules.iter().filter(|m| m.visibility == crate::repository::Visibility::Public).collect::<Vec<_>>(),
        });

        self.render_template("api_docs", &data)
            .unwrap_or_else(|_| "Generate API documentation for this codebase".to_string())
    }

    /// Generate tutorial prompt
    pub fn tutorial_prompt(&self, structure: &CodeStructure, topic: &str) -> String {
        let data = json!({
            "topic": topic,
            "structure": structure,
            "functions": structure.functions,
            "types": structure.types,
        });

        self.render_template("tutorial", &data)
            .unwrap_or_else(|_| format!("Create a tutorial about {} for this codebase", topic))
    }

    /// Generate how-to guide prompt
    pub fn howto_prompt(&self, structure: &CodeStructure, task: &str) -> String {
        let data = json!({
            "task": task,
            "structure": structure,
            "functions": structure.functions,
            "configs": structure.configs,
        });

        self.render_template("howto", &data)
            .unwrap_or_else(|_| format!("Create a how-to guide for: {}", task))
    }

    /// Generate explanation prompt
    pub fn explanation_prompt(&self, structure: &CodeStructure, concept: &str) -> String {
        let data = json!({
            "concept": concept,
            "structure": structure,
            "types": structure.types,
            "modules": structure.modules,
        });

        self.render_template("explanation", &data)
            .unwrap_or_else(|_| format!("Explain the concept of {} in this codebase", concept))
    }

    /// Generate documentation for a specific Diátaxis category
    pub fn category_prompt(
        &self,
        category: &DiátaxisCategory,
        structure: &CodeStructure,
        context: &str,
    ) -> String {
        let template_name = match category {
            DiátaxisCategory::Tutorial => "tutorial",
            DiátaxisCategory::HowTo => "howto",
            DiátaxisCategory::Reference => "reference",
            DiátaxisCategory::Explanation => "explanation",
        };

        let data = json!({
            "context": context,
            "structure": structure,
            "category": category.to_string(),
        });

        self.render_template(template_name, &data)
            .unwrap_or_else(|_| format!("Generate {} documentation: {}", category, context))
    }

    /// Generate summary prompt
    pub fn summary_prompt(&self, structure: &CodeStructure) -> String {
        let data = json!({
            "structure": structure,
            "total_items": structure.item_count(),
        });

        self.render_template("summary", &data)
            .unwrap_or_else(|_| "Provide a summary of this codebase".to_string())
    }

    /// Render a template with data
    fn render_template(
        &self,
        template_name: &str,
        data: &serde_json::Value,
    ) -> Result<String, handlebars::RenderError> {
        self.handlebars.render(template_name, data)
    }

    /// Add a custom template
    pub fn add_template(
        &mut self,
        name: String,
        template: String,
    ) -> Result<(), Box<handlebars::TemplateError>> {
        self.handlebars
            .register_template_string(&name, &template)
            .map_err(Box::new)?;
        self.templates.insert(name, template);
        Ok(())
    }

    /// Get available template names
    pub fn template_names(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }
}

impl Default for PromptTemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in template constants
const BUILTIN_CODE_ANALYSIS_TEMPLATE: &str = r#"
You are an expert software engineer analyzing a codebase. Please analyze the following code structure and provide insights:

**Code Structure Overview:**
- Total items: {{total_items}}
- Modules: {{modules.length}}
- Functions: {{functions.length}} ({{public_functions}} public)
- Types: {{types.length}}
- Configuration files: {{configs.length}}

**Modules:**
{{#each modules}}
- {{name}} ({{visibility}}){{#if documentation}} - {{documentation}}{{/if}}
{{/each}}

**Public Functions:**
{{#each functions}}
{{#if (eq visibility "Public")}}
- {{name}}{{#if is_async}} (async){{/if}}{{#if documentation}} - {{documentation}}{{/if}}
{{/if}}
{{/each}}

**Types:**
{{#each types}}
- {{kind}} {{name}} ({{visibility}}){{#if documentation}} - {{documentation}}{{/if}}
{{/each}}

Please provide:
1. A brief overview of what this codebase does
2. Key architectural patterns used
3. Main functionalities provided
4. Potential areas for improvement
5. Documentation quality assessment

Be concise but comprehensive in your analysis.
"#;

const BUILTIN_TUTORIAL_TEMPLATE: &str = r#"
Create a comprehensive tutorial for "{{topic}}" based on this codebase.

**Available Functions:**
{{#each functions}}
- {{name}}: {{signature}}
{{/each}}

**Available Types:**
{{#each types}}
- {{kind}} {{name}}
{{/each}}

Please create a step-by-step tutorial that:
1. Introduces the concept clearly
2. Provides practical examples
3. Shows how to use the relevant functions and types
4. Includes complete, runnable code examples
5. Explains what the reader will learn

Format as a tutorial following the Diátaxis framework - focus on learning by doing.
"#;

const BUILTIN_HOWTO_TEMPLATE: &str = r#"
Create a practical how-to guide for: "{{task}}"

**Available Functions:**
{{#each functions}}
- {{name}}{{#if documentation}}: {{documentation}}{{/if}}
{{/each}}

**Configuration Files Available:**
{{#each configs}}
- {{path}} ({{format}})
{{/each}}

Please create a how-to guide that:
1. States the goal clearly
2. Lists prerequisites
3. Provides step-by-step instructions
4. Shows practical examples
5. Includes troubleshooting tips

Format as a how-to guide following the Diátaxis framework - focus on solving a specific problem.
"#;

const BUILTIN_REFERENCE_TEMPLATE: &str = r#"
Generate comprehensive API reference documentation for this codebase.

**Functions to Document:**
{{#each functions}}
{{#if (eq visibility "Public")}}
### {{name}}

**Signature:** `{{signature}}`
{{#if documentation}}
**Description:** {{documentation}}
{{/if}}
{{#if is_async}}
**Note:** This is an async function.
{{/if}}

{{/if}}
{{/each}}

**Types to Document:**
{{#each types}}
{{#if (eq visibility "Public")}}
### {{kind}} {{name}}

{{#if documentation}}
**Description:** {{documentation}}
{{/if}}

{{#if fields}}
**Fields:**
{{#each fields}}
- `{{name}}`: {{type_annotation}}{{#if documentation}} - {{documentation}}{{/if}}
{{/each}}
{{/if}}

{{/if}}
{{/each}}

Please provide:
1. Complete API reference with all public functions and types
2. Parameter descriptions
3. Return value descriptions
4. Usage examples for each function
5. Error conditions

Format as reference documentation following the Diátaxis framework - focus on information.
"#;

const BUILTIN_EXPLANATION_TEMPLATE: &str = r#"
Explain the concept of "{{concept}}" in the context of this codebase.

**Relevant Types:**
{{#each types}}
- {{kind}} {{name}}{{#if documentation}} - {{documentation}}{{/if}}
{{/each}}

**Relevant Modules:**
{{#each modules}}
- {{name}}{{#if documentation}} - {{documentation}}{{/if}}
{{/each}}

Please provide an explanation that:
1. Defines the concept clearly
2. Explains why it's important in this context
3. Shows how it's implemented in the codebase
4. Discusses design decisions and trade-offs
5. Relates it to broader software engineering principles

Format as explanation documentation following the Diátaxis framework - focus on understanding.
"#;

const BUILTIN_API_DOCS_TEMPLATE: &str = r#"
Generate API documentation for this codebase.

**Public Functions:**
{{#each functions}}
- {{name}}: {{signature}}{{#if documentation}} - {{documentation}}{{/if}}
{{/each}}

**Public Types:**
{{#each types}}
- {{kind}} {{name}}{{#if documentation}} - {{documentation}}{{/if}}
{{/each}}

**Public Modules:**
{{#each modules}}
- {{name}}{{#if documentation}} - {{documentation}}{{/if}}
{{/each}}

Please create comprehensive API documentation including:
1. Overview of the API
2. Authentication/setup requirements
3. Detailed function descriptions
4. Request/response examples
5. Error handling information
"#;

const BUILTIN_SUMMARY_TEMPLATE: &str = r#"
Provide a concise summary of this codebase.

**Structure:**
- Total items: {{total_items}}
- Modules: {{structure.modules.length}}
- Functions: {{structure.functions.length}}
- Types: {{structure.types.length}}
- Config files: {{structure.configs.length}}

Please provide:
1. One-sentence description of what this codebase does
2. Key features and capabilities
3. Main technologies/frameworks used
4. Target use cases
5. Overall code quality assessment

Keep it concise but informative.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{
        CodeStructure, Function, Module, TypeDefinition, TypeKind, Visibility,
    };

    fn create_test_structure() -> CodeStructure {
        let mut structure = CodeStructure::new();

        structure.functions.push(Function {
            name: "test_function".to_string(),
            signature: "pub fn test_function() -> String".to_string(),
            documentation: Some("A test function".to_string()),
            parameters: Vec::new(),
            return_type: Some("String".to_string()),
            visibility: Visibility::Public,
            is_async: false,
        });

        structure.modules.push(Module {
            name: "test_module".to_string(),
            path: std::path::PathBuf::from("src/test_module.rs"),
            documentation: Some("A test module".to_string()),
            visibility: Visibility::Public,
        });

        structure.types.push(TypeDefinition {
            name: "TestStruct".to_string(),
            kind: TypeKind::Struct,
            documentation: Some("A test struct".to_string()),
            fields: Vec::new(),
            visibility: Visibility::Public,
        });

        structure
    }

    #[test]
    fn test_prompt_library_creation() {
        let library = PromptTemplateLibrary::new();
        let names = library.template_names();

        assert!(names.contains(&"code_analysis".to_string()));
        assert!(names.contains(&"tutorial".to_string()));
        assert!(names.contains(&"howto".to_string()));
        assert!(names.contains(&"reference".to_string()));
        assert!(names.contains(&"explanation".to_string()));
    }

    #[test]
    fn test_code_analysis_prompt() {
        let library = PromptTemplateLibrary::new();
        let structure = create_test_structure();

        let prompt = library.code_analysis_prompt(&structure);
        assert!(prompt.contains("Total items: 3"));
        assert!(prompt.contains("test_function"));
    }

    #[test]
    fn test_api_documentation_prompt() {
        let library = PromptTemplateLibrary::new();
        let structure = create_test_structure();

        let prompt = library.api_documentation_prompt(&structure);
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_tutorial_prompt() {
        let library = PromptTemplateLibrary::new();
        let structure = create_test_structure();

        let prompt = library.tutorial_prompt(&structure, "Getting Started");
        assert!(prompt.contains("Getting Started"));
    }

    #[test]
    fn test_category_prompt() {
        let library = PromptTemplateLibrary::new();
        let structure = create_test_structure();

        let prompt =
            library.category_prompt(&DiátaxisCategory::Tutorial, &structure, "Basic usage");
        assert!(prompt.contains("Basic usage"));
    }

    #[test]
    fn test_custom_template() {
        let mut library = PromptTemplateLibrary::new();

        let custom_template = "Custom template: {{topic}}";
        library
            .add_template("custom".to_string(), custom_template.to_string())
            .unwrap();

        let names = library.template_names();
        assert!(names.contains(&"custom".to_string()));
    }
}
