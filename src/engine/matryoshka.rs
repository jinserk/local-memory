/// Slices a vector to a target dimension and re-normalizes it using L2 normalization.
///
/// Matryoshka embeddings are designed to be truncated while preserving their
/// representational power. Re-normalization is required after slicing to ensure
/// the vector remains on the unit hypersphere, which is important for cosine similarity.
pub fn slice_vector(vec: &[f32], target_dim: usize) -> Result<Vec<f32>, String> {
    if target_dim == 0 {
        return Err("Target dimension must be greater than zero".to_string());
    }

    if target_dim > vec.len() {
        return Err(format!(
            "Target dimension {} is larger than input vector dimension {}",
            target_dim,
            vec.len()
        ));
    }

    let sliced = &vec[..target_dim];

    let sum_squares: f32 = sliced.iter().map(|&x| x * x).sum();
    let norm = sum_squares.sqrt();

    if norm == 0.0 {
        return Ok(sliced.to_vec());
    }

    let normalized = sliced.iter().map(|&x| x / norm).collect();

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_vector_dimension() {
        let vec = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let sliced = slice_vector(&vec, 3).unwrap();
        assert_eq!(sliced.len(), 3);
    }

    #[test]
    fn test_slice_vector_normalization() {
        let vec = vec![1.0, 1.0, 1.0, 1.0];
        let sliced = slice_vector(&vec, 2).unwrap();

        let expected_val = 1.0 / (2.0f32).sqrt();
        assert!((sliced[0] - expected_val).abs() < 1e-6);
        assert!((sliced[1] - expected_val).abs() < 1e-6);

        let norm: f32 = sliced.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_slice_vector_too_large() {
        let vec = vec![1.0, 2.0];
        let result = slice_vector(&vec, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_slice_vector_zero_dim() {
        let vec = vec![1.0, 2.0];
        let result = slice_vector(&vec, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_slice_vector_all_zeros() {
        let vec = vec![0.0, 0.0, 0.0];
        let sliced = slice_vector(&vec, 2).unwrap();
        assert_eq!(sliced, vec![0.0, 0.0]);
    }
}
