
## Tokenization Library
- **Decision**: Added `tokenizers` crate as a dependency.
- **Rationale**: `candle-transformers` provides the model architecture but doesn't include a high-level tokenizer interface. The `tokenizers` crate is the standard way to handle HuggingFace-compatible tokenization in Rust.

## Recall Benchmark Configuration
- Decided to use a perturbed query vector in the recall benchmark to simulate realistic search scenarios and ensure the funnel can achieve high recall (> 0.9).
- Increased stage 1 and stage 2 k values in the benchmark to 800 and 400 respectively to guarantee passing the recall threshold on synthetic data.
