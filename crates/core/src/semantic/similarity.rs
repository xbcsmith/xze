//! Similarity calculations for semantic chunking
//!
//! This module provides functions for calculating similarity between embeddings,
//! including cosine similarity, pairwise similarity calculations, and percentile
//! computations for threshold determination.

use thiserror::Error;

/// Errors that can occur during similarity calculations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SimilarityError {
    /// Vector dimensions do not match
    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// One or both vectors are zero vectors (no magnitude)
    #[error("Cannot calculate similarity for zero vector")]
    ZeroVector,

    /// Invalid value encountered (NaN or infinite)
    #[error("Invalid value in similarity calculation: {0}")]
    InvalidValue(String),
}

/// Calculates the cosine similarity between two vectors
///
/// Cosine similarity measures the cosine of the angle between two vectors,
/// ranging from -1 (opposite) to 1 (identical). A value of 0 indicates
/// orthogonal vectors (no similarity).
///
/// # Arguments
///
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
///
/// Returns the cosine similarity as a value between -1.0 and 1.0
///
/// # Errors
///
/// Returns `SimilarityError::DimensionMismatch` if vectors have different dimensions
/// Returns `SimilarityError::ZeroVector` if either vector has zero magnitude
/// Returns `SimilarityError::InvalidValue` if NaN or infinite values are encountered
///
/// # Examples
///
/// ```
/// use xze_core::semantic::similarity::cosine_similarity;
///
/// let a = vec![1.0, 0.0, 0.0];
/// let b = vec![1.0, 0.0, 0.0];
/// let similarity = cosine_similarity(&a, &b).unwrap();
/// assert!((similarity - 1.0).abs() < 0.0001);
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, SimilarityError> {
    // Check dimensions match
    if a.len() != b.len() {
        return Err(SimilarityError::DimensionMismatch {
            expected: a.len(),
            actual: b.len(),
        });
    }

    // Check for empty vectors
    if a.is_empty() {
        return Err(SimilarityError::DimensionMismatch {
            expected: 0,
            actual: 0,
        });
    }

    // Calculate dot product and magnitudes
    let mut dot_product = 0.0;
    let mut magnitude_a = 0.0;
    let mut magnitude_b = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        magnitude_a += a[i] * a[i];
        magnitude_b += b[i] * b[i];
    }

    // Check for zero vectors
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return Err(SimilarityError::ZeroVector);
    }

    // Calculate cosine similarity
    let similarity = dot_product / (magnitude_a.sqrt() * magnitude_b.sqrt());

    // Check for invalid values
    if similarity.is_nan() {
        return Err(SimilarityError::InvalidValue("NaN".to_string()));
    }
    if similarity.is_infinite() {
        return Err(SimilarityError::InvalidValue("Infinite".to_string()));
    }

    Ok(similarity)
}

/// Calculates pairwise similarities between consecutive embeddings
///
/// Given a sequence of embeddings, this function computes the cosine similarity
/// between each pair of consecutive embeddings (embedding[i] and embedding[i+1]).
/// This is useful for detecting semantic boundaries in a sequence of sentences.
///
/// # Arguments
///
/// * `embeddings` - Slice of embedding vectors
///
/// # Returns
///
/// Returns a vector of similarity scores, with length `embeddings.len() - 1`
///
/// # Errors
///
/// Returns `SimilarityError` if any pairwise similarity calculation fails
///
/// # Examples
///
/// ```
/// use xze_core::semantic::similarity::pairwise_similarities;
///
/// let embeddings = vec![
///     vec![1.0, 0.0],
///     vec![0.9, 0.1],
///     vec![0.0, 1.0],
/// ];
/// let similarities = pairwise_similarities(&embeddings).unwrap();
/// assert_eq!(similarities.len(), 2);
/// ```
pub fn pairwise_similarities(embeddings: &[Vec<f32>]) -> Result<Vec<f32>, SimilarityError> {
    if embeddings.len() < 2 {
        return Ok(Vec::new());
    }

    let mut similarities = Vec::with_capacity(embeddings.len() - 1);

    for i in 0..embeddings.len() - 1 {
        let sim = cosine_similarity(&embeddings[i], &embeddings[i + 1])?;
        similarities.push(sim);
    }

    Ok(similarities)
}

/// Calculates a percentile value from a sorted or unsorted collection
///
/// This function computes the value at a given percentile in a dataset.
/// The percentile is a value below which a given percentage of observations fall.
///
/// # Arguments
///
/// * `values` - Slice of values to compute percentile from
/// * `percentile` - Percentile to calculate (0.0 to 1.0, e.g., 0.25 for 25th percentile)
///
/// # Returns
///
/// Returns the value at the specified percentile
///
/// # Panics
///
/// Panics if `values` is empty or if `percentile` is not between 0.0 and 1.0
///
/// # Examples
///
/// ```
/// use xze_core::semantic::similarity::calculate_percentile;
///
/// let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let median = calculate_percentile(&values, 0.5);
/// assert_eq!(median, 3.0);
/// ```
pub fn calculate_percentile(values: &[f32], percentile: f32) -> f32 {
    assert!(
        !values.is_empty(),
        "Cannot calculate percentile of empty slice"
    );
    assert!(
        (0.0..=1.0).contains(&percentile),
        "Percentile must be between 0.0 and 1.0"
    );

    // Create a sorted copy of the values
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Calculate the index
    let index = (percentile * (sorted.len() - 1) as f32).round() as usize;

    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = cosine_similarity(&a, &b).unwrap();
        assert!((result - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let result = cosine_similarity(&a, &b).unwrap();
        assert!(result.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let result = cosine_similarity(&a, &b).unwrap();
        assert!((result - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_dimension_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = cosine_similarity(&a, &b);
        assert!(matches!(
            result,
            Err(SimilarityError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = cosine_similarity(&a, &b);
        assert!(matches!(result, Err(SimilarityError::ZeroVector)));
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let result = cosine_similarity(&a, &b);
        assert!(matches!(
            result,
            Err(SimilarityError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn test_cosine_similarity_normalized_vectors() {
        // Two normalized vectors at 60 degrees should have similarity = cos(60) = 0.5
        let a = vec![1.0, 0.0];
        let b = vec![0.5, 0.866025]; // cos(60), sin(60)
        let result = cosine_similarity(&a, &b).unwrap();
        assert!((result - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_pairwise_similarities_simple() {
        let embeddings = vec![vec![1.0, 0.0], vec![0.9, 0.1], vec![0.0, 1.0]];
        let similarities = pairwise_similarities(&embeddings).unwrap();
        assert_eq!(similarities.len(), 2);
        assert!(similarities[0] > 0.9); // Very similar
        assert!(similarities[1] < 0.5); // Less similar
    }

    #[test]
    fn test_pairwise_similarities_single_embedding() {
        let embeddings = vec![vec![1.0, 2.0, 3.0]];
        let similarities = pairwise_similarities(&embeddings).unwrap();
        assert_eq!(similarities.len(), 0);
    }

    #[test]
    fn test_pairwise_similarities_empty() {
        let embeddings: Vec<Vec<f32>> = vec![];
        let similarities = pairwise_similarities(&embeddings).unwrap();
        assert_eq!(similarities.len(), 0);
    }

    #[test]
    fn test_pairwise_similarities_identical_consecutive() {
        let embeddings = vec![
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0, 3.0],
            vec![1.0, 2.0, 3.0],
        ];
        let similarities = pairwise_similarities(&embeddings).unwrap();
        assert_eq!(similarities.len(), 2);
        assert!((similarities[0] - 1.0).abs() < 1e-6);
        assert!((similarities[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_percentile_median() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let median = calculate_percentile(&values, 0.5);
        assert_eq!(median, 3.0);
    }

    #[test]
    fn test_calculate_percentile_first_quartile() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let q1 = calculate_percentile(&values, 0.25);
        assert_eq!(q1, 2.0);
    }

    #[test]
    fn test_calculate_percentile_third_quartile() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let q3 = calculate_percentile(&values, 0.75);
        assert_eq!(q3, 4.0);
    }

    #[test]
    fn test_calculate_percentile_unsorted() {
        let values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        let median = calculate_percentile(&values, 0.5);
        assert_eq!(median, 3.0);
    }

    #[test]
    fn test_calculate_percentile_min() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let min = calculate_percentile(&values, 0.0);
        assert_eq!(min, 1.0);
    }

    #[test]
    fn test_calculate_percentile_max() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let max = calculate_percentile(&values, 1.0);
        assert_eq!(max, 5.0);
    }

    #[test]
    #[should_panic(expected = "Cannot calculate percentile of empty slice")]
    fn test_calculate_percentile_empty_panic() {
        let values: Vec<f32> = vec![];
        calculate_percentile(&values, 0.5);
    }

    #[test]
    #[should_panic(expected = "Percentile must be between 0.0 and 1.0")]
    fn test_calculate_percentile_invalid_percentile() {
        let values = vec![1.0, 2.0, 3.0];
        calculate_percentile(&values, 1.5);
    }
}
