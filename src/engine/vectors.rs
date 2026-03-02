use bitvec::prelude::*;

/// Encodes a float vector into a binary quantized (BQ) bit vector.
/// Each positive value becomes a 1, and each non-positive value becomes a 0.
/// Returns a packed bit representation (1 byte = 8 bits).
pub fn encode_bq(vector: &[f32]) -> Vec<u8> {
    let mut bits = BitVec::<u8, Msb0>::with_capacity(vector.len());
    for &val in vector {
        bits.push(val > 0.0);
    }
    bits.into_vec()
}

/// Slices a vector to a smaller dimension (Matryoshka Embedding).
/// This assumes the model was trained with Matryoshka support (like Nomic 1.5).
pub fn slice_vector(vector: &[f32], target_dim: usize) -> Vec<f32> {
    if vector.len() <= target_dim {
        return vector.to_vec();
    }
    
    let sliced = &vector[..target_dim];
    
    // Re-normalize the sliced vector
    let norm: f32 = sliced.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        sliced.iter().map(|x| x / norm).collect()
    } else {
        sliced.to_vec()
    }
}
