//! Sentence splitting functionality for semantic chunking
//!
//! This module provides tools for splitting text into sentences while preserving
//! code blocks and handling common abbreviations correctly.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Common abbreviations that should not trigger sentence boundaries
static ABBREVIATIONS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Sr.", "Jr.", "vs.", "etc.", "e.g.", "i.e.", "Ph.D.",
        "M.D.", "U.S.", "U.K.", "Inc.", "Ltd.", "Corp.", "Co.",
    ]
});

/// Pattern for detecting code blocks in Markdown
static CODE_BLOCK_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"```[\s\S]*?```|`[^`]+`").expect("Failed to compile code block pattern regex")
});

/// Sentence splitter that preserves code blocks and handles abbreviations
///
/// The splitter intelligently breaks text into sentences while:
/// - Preserving code blocks (both inline and fenced)
/// - Not breaking on common abbreviations
/// - Filtering out very short fragments
///
/// # Examples
///
/// ```
/// use xze_core::semantic::splitter::SentenceSplitter;
///
/// let splitter = SentenceSplitter::new(10);
/// let text = "This is a sentence. This is another sentence.";
/// let sentences = splitter.split(text);
///
/// assert_eq!(sentences.len(), 2);
/// assert_eq!(sentences[0], "This is a sentence.");
/// ```
#[derive(Debug, Clone)]
pub struct SentenceSplitter {
    /// Minimum length (in characters) for a valid sentence
    min_sentence_length: usize,
}

impl SentenceSplitter {
    /// Creates a new sentence splitter with the specified minimum sentence length
    ///
    /// # Arguments
    ///
    /// * `min_sentence_length` - Minimum number of characters for a valid sentence
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::splitter::SentenceSplitter;
    ///
    /// let splitter = SentenceSplitter::new(15);
    /// assert_eq!(splitter.min_sentence_length(), 15);
    /// ```
    pub fn new(min_sentence_length: usize) -> Self {
        Self {
            min_sentence_length,
        }
    }

    /// Returns the minimum sentence length
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::splitter::SentenceSplitter;
    ///
    /// let splitter = SentenceSplitter::new(20);
    /// assert_eq!(splitter.min_sentence_length(), 20);
    /// ```
    pub fn min_sentence_length(&self) -> usize {
        self.min_sentence_length
    }

    /// Splits text into sentences while preserving code blocks
    ///
    /// This method:
    /// 1. Extracts and preserves code blocks
    /// 2. Protects common abbreviations from being split
    /// 3. Splits on sentence boundaries
    /// 4. Restores code blocks and abbreviations
    /// 5. Filters out sentences shorter than the minimum length
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to split
    ///
    /// # Returns
    ///
    /// A vector of sentences as strings
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::splitter::SentenceSplitter;
    ///
    /// let splitter = SentenceSplitter::new(5);
    /// let text = "First sentence. Second sentence. Third sentence.";
    /// let sentences = splitter.split(text);
    ///
    /// assert_eq!(sentences.len(), 3);
    /// ```
    pub fn split(&self, text: &str) -> Vec<String> {
        if text.trim().is_empty() {
            return Vec::new();
        }

        // Step 1: Extract and preserve code blocks
        let (text_without_code, code_blocks) = self.extract_code_blocks(text);

        // Step 2: Protect abbreviations
        let protected_text = self.protect_abbreviations(&text_without_code);

        // Step 3: Split on sentence boundaries
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();
        let chars: Vec<char> = protected_text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];
            current_sentence.push(ch);

            // Check if this is sentence-ending punctuation
            if ch == '.' || ch == '!' || ch == '?' {
                // Look ahead to see if we should end the sentence
                let mut should_split = false;

                // Check if this is the end of text
                if i == chars.len() - 1 {
                    should_split = true;
                } else {
                    // Look at next character(s)
                    let mut j = i + 1;
                    // Skip whitespace
                    while j < chars.len() && chars[j].is_whitespace() {
                        if chars[j] == '\n' || chars[j] == '\r' {
                            should_split = true;
                            break;
                        }
                        j += 1;
                    }
                    // If we found an uppercase letter after whitespace, split
                    if j < chars.len() && chars[j].is_uppercase() {
                        should_split = true;
                    }
                }

                if should_split {
                    let sentence = current_sentence.trim().to_string();
                    if !sentence.is_empty() {
                        sentences.push(sentence);
                    }
                    current_sentence.clear();
                }
            }

            i += 1;
        }

        // Add any remaining text as the last sentence
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
        }

        // Step 4: Restore abbreviations and code blocks
        sentences = sentences
            .into_iter()
            .map(|s| self.restore_abbreviations(&s))
            .map(|s| self.restore_code_blocks(&s, &code_blocks))
            .collect();

        // Step 5: Filter short sentences
        sentences
            .into_iter()
            .filter(|s| s.chars().count() >= self.min_sentence_length)
            .collect()
    }

    /// Extracts code blocks from text and replaces them with placeholders
    ///
    /// Returns a tuple of (text with placeholders, map of placeholders to code blocks)
    fn extract_code_blocks(&self, text: &str) -> (String, HashMap<String, String>) {
        let mut code_blocks = HashMap::new();
        let mut result = text.to_string();

        for (index, cap) in CODE_BLOCK_PATTERN.captures_iter(text).enumerate() {
            let code = cap.get(0).unwrap().as_str();
            let placeholder = format!("__CODE_BLOCK_{}__", index);
            code_blocks.insert(placeholder.clone(), code.to_string());
            result = result.replacen(code, &placeholder, 1);
        }

        (result, code_blocks)
    }

    /// Restores code blocks from placeholders
    fn restore_code_blocks(&self, text: &str, code_blocks: &HashMap<String, String>) -> String {
        let mut result = text.to_string();

        for (placeholder, code) in code_blocks {
            if result.contains(placeholder) {
                result = result.replace(placeholder, code);
            }
        }

        result
    }

    /// Protects abbreviations by replacing periods with a placeholder
    fn protect_abbreviations(&self, text: &str) -> String {
        let mut result = text.to_string();
        for abbr in ABBREVIATIONS.iter() {
            let protected = abbr.replace('.', "{{DOT}}");
            result = result.replace(abbr, &protected);
        }
        result
    }

    /// Restores abbreviations by replacing placeholders with periods
    fn restore_abbreviations(&self, text: &str) -> String {
        text.replace("{{DOT}}", ".")
    }
}

impl Default for SentenceSplitter {
    /// Creates a sentence splitter with default settings (minimum length: 10)
    ///
    /// # Examples
    ///
    /// ```
    /// use xze_core::semantic::splitter::SentenceSplitter;
    ///
    /// let splitter = SentenceSplitter::default();
    /// assert_eq!(splitter.min_sentence_length(), 10);
    /// ```
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_simple_sentences() {
        let splitter = SentenceSplitter::new(5);
        let text = "This is the first sentence. This is the second sentence. And a third.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "This is the first sentence.");
        assert_eq!(sentences[1], "This is the second sentence.");
        assert_eq!(sentences[2], "And a third.");
    }

    #[test]
    fn test_split_with_exclamation_and_question() {
        let splitter = SentenceSplitter::new(5);
        let text = "What is this? It is amazing! Really great.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "What is this?");
        assert_eq!(sentences[1], "It is amazing!");
        assert_eq!(sentences[2], "Really great.");
    }

    #[test]
    fn test_split_preserves_inline_code() {
        let splitter = SentenceSplitter::new(5);
        let text = "Use the `config.yaml` file. Then run `cargo build`.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].contains("`config.yaml`"));
        assert!(sentences[1].contains("`cargo build`"));
    }

    #[test]
    fn test_split_preserves_fenced_code_blocks() {
        let splitter = SentenceSplitter::new(5);
        let text = "Here is code:\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\nThat was the code.";
        let sentences = splitter.split(text);

        assert!(sentences.len() >= 1);
        // Check that code block is preserved somewhere
        let combined = sentences.join(" ");
        assert!(combined.contains("```rust"));
        assert!(combined.contains("println!"));
    }

    #[test]
    fn test_split_handles_abbreviations() {
        let splitter = SentenceSplitter::new(5);
        let text = "Dr. Smith is here. He works for Inc. Corporation.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].contains("Dr. Smith"));
        assert!(sentences[1].contains("Inc. Corporation"));
    }

    #[test]
    fn test_split_handles_multiple_abbreviations() {
        let splitter = SentenceSplitter::new(5);
        let text = "Prof. Johnson, Ph.D., met Mr. Brown, Jr. They discussed U.S. policy.";
        let sentences = splitter.split(text);

        assert!(sentences.len() >= 1);
        assert!(sentences[0].contains("Ph.D."));
        // The second sentence should preserve abbreviations
        let combined = sentences.join(" ");
        assert!(combined.contains("U.S."));
    }

    #[test]
    fn test_split_filters_short_fragments() {
        let splitter = SentenceSplitter::new(15);
        let text = "This is a long enough sentence. Hi. Another long sentence here.";
        let sentences = splitter.split(text);

        // "Hi." should be filtered out as it's less than 15 characters
        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].contains("long enough"));
        assert!(sentences[1].contains("Another long"));
    }

    #[test]
    fn test_split_empty_string() {
        let splitter = SentenceSplitter::new(10);
        let sentences = splitter.split("");

        assert_eq!(sentences.len(), 0);
    }

    #[test]
    fn test_split_whitespace_only() {
        let splitter = SentenceSplitter::new(10);
        let sentences = splitter.split("   \n\t  ");

        assert_eq!(sentences.len(), 0);
    }

    #[test]
    fn test_split_single_sentence() {
        let splitter = SentenceSplitter::new(5);
        let text = "This is a single sentence.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 1);
        assert_eq!(sentences[0], "This is a single sentence.");
    }

    #[test]
    fn test_split_no_ending_punctuation() {
        let splitter = SentenceSplitter::new(5);
        let text = "This is a sentence without ending punctuation";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 1);
        assert_eq!(
            sentences[0],
            "This is a sentence without ending punctuation"
        );
    }

    #[test]
    fn test_default_splitter() {
        let splitter = SentenceSplitter::default();
        assert_eq!(splitter.min_sentence_length(), 10);
    }

    #[test]
    fn test_split_with_newlines() {
        let splitter = SentenceSplitter::new(5);
        let text = "First sentence.\nSecond sentence.\n\nThird sentence.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn test_multiple_code_blocks() {
        let splitter = SentenceSplitter::new(5);
        let text = "Use `var1` and `var2`. Then call `func()`.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].contains("`var1`"));
        assert!(sentences[0].contains("`var2`"));
        assert!(sentences[1].contains("`func()`"));
    }

    #[test]
    fn test_sentence_with_etc() {
        let splitter = SentenceSplitter::new(5);
        let text = "We need files, folders, etc. for the project.";
        let sentences = splitter.split(text);

        assert_eq!(sentences.len(), 1);
        assert!(sentences[0].contains("etc."));
    }

    #[test]
    fn test_min_length_boundary() {
        let splitter = SentenceSplitter::new(10);
        let text = "Exactly10!. This is longer sentence.";
        let sentences = splitter.split(text);

        // First sentence is exactly 10 chars (including punctuation)
        assert_eq!(sentences.len(), 2);
    }
}
