use bitvec::prelude::*;

pub fn encode_bq(vector: &[f32]) -> Vec<u8> {
    let mut bv = BitVec::<u8, Msb0>::with_capacity(vector.len());
    for &val in vector {
        bv.push(val > 0.0);
    }
    bv.into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_bq_basic() {
        let vec = vec![1.0, -1.0, 0.5, 0.0, -0.5, 2.0, -2.0, 0.1];
        let encoded = encode_bq(&vec);
        assert_eq!(encoded, vec![0xA5]);
    }

    #[test]
    fn test_encode_bq_padding() {
        let vec = vec![1.0, -1.0, 1.0];
        let encoded = encode_bq(&vec);
        assert_eq!(encoded, vec![0xA0]);
    }

    #[test]
    fn test_encode_bq_multiple_bytes() {
        let mut vec = vec![-1.0; 12];
        vec[0] = 1.0;
        vec[7] = 1.0;
        vec[8] = 1.0;
        let encoded = encode_bq(&vec);
        assert_eq!(encoded, vec![0x81, 0x80]);
    }
}
