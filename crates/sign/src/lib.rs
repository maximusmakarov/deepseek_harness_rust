#![forbid(unsafe_code)]
//! Ed25519 signatures for release artifacts and plugin manifests.
//!
//! Keys and signatures are lower-case hexadecimal strings so they can be
//! stored in YAML manifests, shell variables and GitHub secrets.

use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignError {
    #[error("signing key must be a 32-byte hex string")]
    SecretFormat,

    #[error("public key must be a 32-byte hex string")]
    PublicFormat,

    #[error("signature must be a 64-byte hex string")]
    SignatureFormat,

    #[error("invalid Ed25519 key: {0}")]
    Key(String),

    #[error("signature does not verify")]
    Invalid,
}

/// Generates a new key pair as `(public_hex, secret_hex)`.
pub fn keygen() -> Result<(String, String), SignError> {
    let mut seed = [0u8; 32];
    SystemRandom::new()
        .fill(&mut seed)
        .map_err(|error| SignError::Key(error.to_string()))?;
    let key = key_pair(&seed)?;
    Ok((hex_encode(key.public_key().as_ref()), hex_encode(&seed)))
}

/// Derives the public key for a secret key.
pub fn public_key(secret_hex: &str) -> Result<String, SignError> {
    let seed = decode::<32>(secret_hex).ok_or(SignError::SecretFormat)?;
    let key = key_pair(&seed)?;
    Ok(hex_encode(key.public_key().as_ref()))
}

/// Signs an arbitrary message, returning the signature in hex.
pub fn sign_bytes(secret_hex: &str, message: &[u8]) -> Result<String, SignError> {
    let seed = decode::<32>(secret_hex).ok_or(SignError::SecretFormat)?;
    let key = key_pair(&seed)?;
    Ok(hex_encode(key.sign(message).as_ref()))
}

/// Verifies a hex signature over a message with a hex public key.
pub fn verify_bytes(
    public_hex: &str,
    message: &[u8],
    signature_hex: &str,
) -> Result<(), SignError> {
    let public = decode::<32>(public_hex).ok_or(SignError::PublicFormat)?;
    let signature = decode::<64>(signature_hex).ok_or(SignError::SignatureFormat)?;
    UnparsedPublicKey::new(&ED25519, public)
        .verify(message, &signature)
        .map_err(|_| SignError::Invalid)
}

/// Checks that a string is a 32-byte hexadecimal public key.
pub fn validate_public_key(hex: &str) -> Result<(), SignError> {
    decode::<32>(hex).map(|_| ()).ok_or(SignError::PublicFormat)
}

/// Checks that a string is a 32-byte hexadecimal secret key.
pub fn validate_secret_key(hex: &str) -> Result<(), SignError> {
    decode::<32>(hex).map(|_| ()).ok_or(SignError::SecretFormat)
}

fn key_pair(seed: &[u8; 32]) -> Result<Ed25519KeyPair, SignError> {
    Ed25519KeyPair::from_seed_unchecked(seed).map_err(|error| SignError::Key(error.to_string()))
}

fn decode<const N: usize>(value: &str) -> Option<[u8; N]> {
    let value = value.trim();
    if value.len() != N * 2 {
        return None;
    }
    let mut bytes = [0u8; N];
    for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let text = std::str::from_utf8(chunk).ok()?;
        bytes[index] = u8::from_str_radix(text, 16).ok()?;
    }
    Some(bytes)
}

#[must_use]
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_and_verifies() {
        let (public, secret) = keygen().expect("keygen");
        assert_eq!(public_key(&secret).expect("public"), public);
        let message = b"dshr release";
        let signature = sign_bytes(&secret, message).expect("sign");
        verify_bytes(&public, message, &signature).expect("verify");
        assert!(matches!(
            verify_bytes(&public, b"tampered", &signature),
            Err(SignError::Invalid)
        ));
    }

    #[test]
    fn rejects_other_key_and_bad_hex() {
        let (public, secret) = keygen().expect("keygen");
        let (other, _other_secret) = keygen().expect("keygen");
        let signature = sign_bytes(&secret, b"payload").expect("sign");
        assert!(verify_bytes(&other, b"payload", &signature).is_err());
        assert!(verify_bytes(&public, b"payload", "deadbeef").is_err());
        assert!(matches!(
            sign_bytes("not-hex", b"payload"),
            Err(SignError::SecretFormat)
        ));
        assert!(validate_public_key(&public).is_ok());
        assert!(validate_secret_key(&secret).is_ok());
    }
}
