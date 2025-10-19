//! Cross-reference linking for documentation

use crate::types::DiátaxisCategory;
use std::collections::HashMap;

use super::Document;

/// Cross-reference generator for linking related documents
pub struct CrossReferenceGenerator {
    link_strategy: LinkStrategy,
}

/// Strategy for generating cross-reference links
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStrategy {
    /// Link related documents within same category
    SameCategory,
    /// Link complementary documents across categories
    Complementary,
    /// Link all related documents
    All,
}

impl Default for LinkStrategy {
    fn default() -> Self {
        Self::Complementary
    }
}

impl CrossReferenceGenerator {
    /// Create a new cross-reference generator
    pub fn new(link_strategy: LinkStrategy) -> Self {
        Self { link_strategy }
    }

    /// Generate cross-references for all documents
    pub fn generate_cross_references(
        &self,
        documents: &[Document],
    ) -> HashMap<String, Vec<CrossReference>> {
        let mut references: HashMap<String, Vec<CrossReference>> = HashMap::new();

        for doc in documents {
            let refs = self.find_related_documents(doc, documents);
            references.insert(doc.title.clone(), refs);
        }

        references
    }

    /// Find related documents for a given document
    fn find_related_documents(&self, doc: &Document, all_docs: &[Document]) -> Vec<CrossReference> {
        let mut related = Vec::new();

        for other in all_docs {
            if other.title == doc.title {
                continue; // Skip self
            }

            if self.should_link(&doc.category, &other.category) {
                related.push(CrossReference {
                    title: other.title.clone(),
                    category: other.category.clone(),
                    path: self.generate_relative_path(&doc.category, &other.category, other),
                    relationship: self.determine_relationship(&doc.category, &other.category),
                });
            }
        }

        related
    }

    /// Determine if two categories should be linked
    fn should_link(&self, from: &DiátaxisCategory, to: &DiátaxisCategory) -> bool {
        match self.link_strategy {
            LinkStrategy::SameCategory => from == to,
            LinkStrategy::Complementary => self.are_complementary(from, to),
            LinkStrategy::All => true,
        }
    }

    /// Check if two categories are complementary
    fn are_complementary(&self, from: &DiátaxisCategory, to: &DiátaxisCategory) -> bool {
        match (from, to) {
            // Tutorials link to How-Tos and Reference
            (DiátaxisCategory::Tutorial, DiátaxisCategory::HowTo) => true,
            (DiátaxisCategory::Tutorial, DiátaxisCategory::Reference) => true,

            // How-Tos link to Tutorials and Reference
            (DiátaxisCategory::HowTo, DiátaxisCategory::Tutorial) => true,
            (DiátaxisCategory::HowTo, DiátaxisCategory::Reference) => true,

            // Reference links to Explanations
            (DiátaxisCategory::Reference, DiátaxisCategory::Explanation) => true,

            // Explanations link to Tutorials and Reference
            (DiátaxisCategory::Explanation, DiátaxisCategory::Tutorial) => true,
            (DiátaxisCategory::Explanation, DiátaxisCategory::Reference) => true,

            _ => false,
        }
    }

    /// Determine the relationship between two categories
    fn determine_relationship(
        &self,
        from: &DiátaxisCategory,
        to: &DiátaxisCategory,
    ) -> Relationship {
        match (from, to) {
            (DiátaxisCategory::Tutorial, DiátaxisCategory::HowTo) => Relationship::NextSteps,
            (DiátaxisCategory::Tutorial, DiátaxisCategory::Reference) => Relationship::Reference,
            (DiátaxisCategory::HowTo, DiátaxisCategory::Tutorial) => Relationship::LearnMore,
            (DiátaxisCategory::HowTo, DiátaxisCategory::Reference) => Relationship::Reference,
            (DiátaxisCategory::Reference, DiátaxisCategory::Explanation) => {
                Relationship::DeepDive
            }
            (DiátaxisCategory::Explanation, DiátaxisCategory::Tutorial) => Relationship::HandsOn,
            (DiátaxisCategory::Explanation, DiátaxisCategory::Reference) => {
                Relationship::TechnicalDetails
            }
            _ => Relationship::Related,
        }
    }

    /// Generate relative path between documents
    fn generate_relative_path(
        &self,
        from_category: &DiátaxisCategory,
        to_category: &DiátaxisCategory,
        to_doc: &Document,
    ) -> String {
        let from_dir = self.category_to_dir(from_category);
        let to_dir = self.category_to_dir(to_category);

        let filename = to_doc
            .file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if from_dir == to_dir {
            // Same directory
            filename.to_string()
        } else {
            // Different directory - use relative path
            format!("../{}/{}", to_dir, filename)
        }
    }

    /// Convert category to directory name
    fn category_to_dir(&self, category: &DiátaxisCategory) -> &str {
        match category {
            DiátaxisCategory::Tutorial => "tutorials",
            DiátaxisCategory::HowTo => "how_to",
            DiátaxisCategory::Reference => "reference",
            DiátaxisCategory::Explanation => "explanations",
        }
    }

    /// Add cross-references section to document content
    pub fn add_cross_references(&self, content: &str, references: &[CrossReference]) -> String {
        if references.is_empty() {
            return content.to_string();
        }

        let mut result = content.to_string();

        // Ensure proper spacing before section
        if !result.ends_with("\n\n") {
            if result.ends_with('\n') {
                result.push('\n');
            } else {
                result.push_str("\n\n");
            }
        }

        result.push_str("---\n\n");
        result.push_str("## Related Documentation\n\n");

        // Group by relationship
        let mut by_relationship: HashMap<Relationship, Vec<&CrossReference>> = HashMap::new();
        for cref in references {
            by_relationship
                .entry(cref.relationship)
                .or_insert_with(Vec::new)
                .push(cref);
        }

        // Output grouped references
        for (relationship, refs) in by_relationship.iter() {
            if !refs.is_empty() {
                result.push_str(&format!("### {}\n\n", relationship.description()));

                for cref in refs {
                    result.push_str(&format!(
                        "- [{}]({}) ({})\n",
                        cref.title, cref.path, cref.category
                    ));
                }

                result.push('\n');
            }
        }

        result
    }
}

/// Cross-reference to another document
#[derive(Debug, Clone)]
pub struct CrossReference {
    /// Title of the referenced document
    pub title: String,
    /// Category of the referenced document
    pub category: DiátaxisCategory,
    /// Relative path to the document
    pub path: String,
    /// Relationship type
    pub relationship: Relationship,
}

/// Relationship between documents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relationship {
    /// Next steps after learning
    NextSteps,
    /// Learn more about basics
    LearnMore,
    /// Reference material
    Reference,
    /// Deep dive into concepts
    DeepDive,
    /// Hands-on practice
    HandsOn,
    /// Technical details
    TechnicalDetails,
    /// General related content
    Related,
}

impl Relationship {
    /// Get description for relationship type
    pub fn description(&self) -> &str {
        match self {
            Self::NextSteps => "Next Steps",
            Self::LearnMore => "Learn More",
            Self::Reference => "Reference Material",
            Self::DeepDive => "Deep Dive",
            Self::HandsOn => "Hands-On Practice",
            Self::TechnicalDetails => "Technical Details",
            Self::Related => "Related Content",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documentation::Document;
    use std::path::PathBuf;

    fn create_test_document(category: DiátaxisCategory, title: &str, filename: &str) -> Document {
        Document::new(
            category.clone(),
            title.to_string(),
            "Test content".to_string(),
            PathBuf::from(format!(
                "{}/{}",
                match category {
                    DiátaxisCategory::Tutorial => "tutorials",
                    DiátaxisCategory::HowTo => "how_to",
                    DiátaxisCategory::Reference => "reference",
                    DiátaxisCategory::Explanation => "explanations",
                },
                filename
            )),
        )
    }

    #[test]
    fn test_cross_reference_generator_creation() {
        let generator = CrossReferenceGenerator::new(LinkStrategy::Complementary);
        assert_eq!(generator.link_strategy, LinkStrategy::Complementary);
    }

    #[test]
    fn test_complementary_linking() {
        let generator = CrossReferenceGenerator::new(LinkStrategy::Complementary);

        // Tutorial should link to How-To and Reference
        assert!(generator.are_complementary(&DiátaxisCategory::Tutorial, &DiátaxisCategory::HowTo));
        assert!(
            generator.are_complementary(&DiátaxisCategory::Tutorial, &DiátaxisCategory::Reference)
        );

        // Tutorial should not link to Explanation
        assert!(!generator
            .are_complementary(&DiátaxisCategory::Tutorial, &DiátaxisCategory::Explanation));
    }

    #[test]
    fn test_generate_cross_references() {
        let generator = CrossReferenceGenerator::new(LinkStrategy::Complementary);

        let docs = vec![
            create_test_document(DiátaxisCategory::Tutorial, "Tutorial 1", "tutorial1.md"),
            create_test_document(DiátaxisCategory::HowTo, "How-To 1", "howto1.md"),
            create_test_document(DiátaxisCategory::Reference, "API Reference", "api.md"),
        ];

        let references = generator.generate_cross_references(&docs);

        // Tutorial should have references
        let tutorial_refs = references.get("Tutorial 1").unwrap();
        assert!(!tutorial_refs.is_empty());

        // Should link to How-To and Reference
        assert!(tutorial_refs.iter().any(|r| r.title == "How-To 1"));
        assert!(tutorial_refs.iter().any(|r| r.title == "API Reference"));
    }

    #[test]
    fn test_relative_path_generation() {
        let generator = CrossReferenceGenerator::new(LinkStrategy::Complementary);

        let tutorial = create_test_document(DiátaxisCategory::Tutorial, "Tutorial", "tutorial.md");
        let howto = create_test_document(DiátaxisCategory::HowTo, "How-To", "howto.md");

        let path = generator.generate_relative_path(&tutorial.category, &howto.category, &howto);

        assert_eq!(path, "../how_to/howto.md");
    }

    #[test]
    fn test_same_category_path() {
        let generator = CrossReferenceGenerator::new(LinkStrategy::Complementary);

        let doc1 = create_test_document(DiátaxisCategory::Tutorial, "Tutorial 1", "tutorial1.md");
        let doc2 = create_test_document(DiátaxisCategory::Tutorial, "Tutorial 2", "tutorial2.md");

        let path = generator.generate_relative_path(&doc1.category, &doc2.category, &doc2);

        assert_eq!(path, "tutorial2.md");
    }

    #[test]
    fn test_relationship_descriptions() {
        assert_eq!(Relationship::NextSteps.description(), "Next Steps");
        assert_eq!(Relationship::LearnMore.description(), "Learn More");
        assert_eq!(Relationship::Reference.description(), "Reference Material");
        assert_eq!(Relationship::DeepDive.description(), "Deep Dive");
    }

    #[test]
    fn test_add_cross_references() {
        let generator = CrossReferenceGenerator::new(LinkStrategy::Complementary);

        let content = "# Test Document\n\nSome content here.\n";

        let references = vec![CrossReference {
            title: "Related Doc".to_string(),
            category: DiátaxisCategory::HowTo,
            path: "../how_to/related.md".to_string(),
            relationship: Relationship::NextSteps,
        }];

        let result = generator.add_cross_references(content, &references);

        assert!(result.contains("## Related Documentation"));
        assert!(result.contains("Related Doc"));
        assert!(result.contains("../how_to/related.md"));
    }
}
