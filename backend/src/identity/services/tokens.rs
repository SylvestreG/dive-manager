use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// A freshly generated secret token: the plaintext goes to the user (by URL
/// or email), only the hash is stored (foundation design §7).
pub struct GeneratedToken {
    pub token: String,
    pub hash: Vec<u8>,
}

pub fn generate() -> GeneratedToken {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash(&token);
    GeneratedToken { token, hash }
}

pub fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_unique_and_hash_matches() {
        let a = generate();
        let b = generate();
        assert_ne!(a.token, b.token);
        assert_eq!(a.hash, hash(&a.token));
        assert_ne!(a.hash, b.hash);
        assert_eq!(a.hash.len(), 32);
    }
}
