use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Embedding: vector representation of a code artifact
// ---------------------------------------------------------------------------

/// A dense vector embedding of a code artifact for semantic similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The raw vector values.
    pub values: Vec<f32>,
    /// Dimensionality of the embedding.
    pub dimensions: usize,
    /// The model/method used to generate this embedding.
    pub model: EmbeddingModel,
}

impl Embedding {
    /// Create a new embedding from a vector of f32 values.
    pub fn new(values: Vec<f32>, model: EmbeddingModel) -> Self {
        let dimensions = values.len();
        Self {
            values,
            dimensions,
            model,
        }
    }

    /// Compute cosine similarity between two embeddings.
    /// Returns a value in [-1.0, 1.0] where 1.0 = identical direction.
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        assert_eq!(
            self.dimensions, other.dimensions,
            "Embedding dimensions must match"
        );

        let dot: f32 = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum();
        let norm_a: f32 = self.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.values.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }

    /// Compute Euclidean distance between two embeddings.
    pub fn euclidean_distance(&self, other: &Embedding) -> f32 {
        assert_eq!(
            self.dimensions, other.dimensions,
            "Embedding dimensions must match"
        );

        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Normalize the embedding to unit length (L2 norm = 1).
    pub fn normalize(&mut self) {
        let norm: f32 = self.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut self.values {
                *v /= norm;
            }
        }
    }
}

/// The model or method used to generate an embedding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingModel {
    /// Simple bag-of-words TF-IDF (prototype only).
    BagOfWords,
    /// Placeholder for a transformer model.
    Transformer(String),
    /// External API (e.g., OpenAI, Cohere).
    ExternalApi(String),
}

// ---------------------------------------------------------------------------
// Simple bag-of-words embedding generator (prototype)
// ---------------------------------------------------------------------------

/// A simple embedding generator using bag-of-words with term frequency.
/// This is a prototype implementation; production would use a transformer.
pub struct BagOfWordsEmbedder {
    /// Fixed vocabulary for consistent dimensionality.
    vocabulary: Vec<String>,
}

impl BagOfWordsEmbedder {
    /// Create a new embedder with a fixed vocabulary.
    pub fn new(vocabulary: Vec<String>) -> Self {
        Self { vocabulary }
    }

    /// Build a vocabulary from a corpus of documents.
    pub fn from_corpus(documents: &[&str], max_vocab_size: usize) -> Self {
        use std::collections::HashMap;

        let mut word_counts: HashMap<String, usize> = HashMap::new();
        for doc in documents {
            for word in doc.split_whitespace() {
                let word = word
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<String>();
                if !word.is_empty() {
                    *word_counts.entry(word).or_insert(0) += 1;
                }
            }
        }

        let mut sorted: Vec<_> = word_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(max_vocab_size);

        let vocabulary = sorted.into_iter().map(|(word, _)| word).collect();
        Self { vocabulary }
    }

    /// Generate an embedding for a text string.
    pub fn embed(&self, text: &str) -> Embedding {
        use std::collections::HashMap;

        let mut word_counts: HashMap<String, f32> = HashMap::new();
        let total_words = text.split_whitespace().count() as f32;

        for word in text.split_whitespace() {
            let word = word
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>();
            if !word.is_empty() {
                *word_counts.entry(word).or_insert(0.0) += 1.0;
            }
        }

        let values: Vec<f32> = self
            .vocabulary
            .iter()
            .map(|vocab_word| {
                word_counts.get(vocab_word).copied().unwrap_or(0.0) / total_words.max(1.0)
            })
            .collect();

        let mut emb = Embedding::new(values, EmbeddingModel::BagOfWords);
        emb.normalize();
        emb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = Embedding::new(vec![1.0, 0.0, 0.0], EmbeddingModel::BagOfWords);
        let b = Embedding::new(vec![1.0, 0.0, 0.0], EmbeddingModel::BagOfWords);
        let sim = a.cosine_similarity(&b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = Embedding::new(vec![1.0, 0.0, 0.0], EmbeddingModel::BagOfWords);
        let b = Embedding::new(vec![0.0, 1.0, 0.0], EmbeddingModel::BagOfWords);
        let sim = a.cosine_similarity(&b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let mut emb = Embedding::new(vec![3.0, 4.0], EmbeddingModel::BagOfWords);
        emb.normalize();
        let norm: f32 = emb.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_bag_of_words_embedder() {
        let corpus = &["fn main pub struct", "use crate import mod"];
        let embedder = BagOfWordsEmbedder::from_corpus(corpus, 10);
        let emb = embedder.embed("fn main hello");
        assert_eq!(emb.dimensions, embedder.vocabulary.len());
    }
}
