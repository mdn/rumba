use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::Value;

/// Returns a proper token from a json file path.
pub fn token_from_file(json_path: &str) -> String {
    let json_str = std::fs::read_to_string(json_path).unwrap();
    token_from_json(&json_str)
}

pub fn token_from_claim(claims: &Value) -> String {
    token_from_claim_and_pem(claims, Pem::Valid)
}

/// Returns an invalid token from a json file path.
pub fn invalid_token_from_json_file(json_path: &str) -> String {
    let json_str = std::fs::read_to_string(json_path).unwrap();
    invalid_token_from_json(&json_str)
}

/// Returns a proper token from a json string.
pub fn token_from_json(json_str: &str) -> String {
    token_from_json_string_and_pem(json_str, Pem::Valid)
}
/// Returns an invalid token from a json string.
pub fn invalid_token_from_json(json_str: &str) -> String {
    token_from_json_string_and_pem(json_str, Pem::Invalid)
}

enum Pem {
    Valid,
    Invalid,
}

fn token_from_json_string_and_pem(json_str: &str, pem_variant: Pem) -> String {
    let claims: Value = serde_json::from_str(json_str).unwrap();
    token_from_claim_and_pem(&claims, pem_variant)
}

fn token_from_claim_and_pem(claims: &Value, pem_variant: Pem) -> String {
    let header = Header {
        kid: Some("TEST_KEY".to_owned()),
        alg: Algorithm::RS256,
        ..Default::default()
    };

    let encoding_key = match pem_variant {
        Pem::Valid => EncodingKey::from_rsa_pem(include_bytes!("../data/rumba-test.pem")).unwrap(),
        Pem::Invalid => {
            EncodingKey::from_rsa_pem(include_bytes!("../data/rumba-test-invalid.pem")).unwrap()
        }
    };

    encode(&header, &claims, &encoding_key).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn verify() {
        let proper = vec![
            (
                "tests/data/set_tokens/test/set_token_subscription_state_change_to_5m.txt",
                "tests/data/set_tokens/set_token_subscription_state_change_to_5m.json",
            ),
            (
                "tests/data/set_tokens/test/set_token_delete_user.txt",
                "tests/data/set_tokens/set_token_delete_user.json",
            ),
            (
                "tests/data/set_tokens/test/set_token_profile_change.txt",
                "tests/data/set_tokens/set_token_profile_change.json",
            ),
            (
                "tests/data/set_tokens/test/set_token_subscription_state_change_to_10m.txt",
                "tests/data/set_tokens/set_token_subscription_state_change_to_10m.json",
            ),
            (
                "tests/data/set_tokens/test/set_token_subscription_state_change_to_5m.txt",
                "tests/data/set_tokens/set_token_subscription_state_change_to_5m.json",
            ),
            (
                "tests/data/set_tokens/test/set_token_subscription_state_change_to_core.txt",
                "tests/data/set_tokens/set_token_subscription_state_change_to_core.json",
            ),
            (
                "tests/data/set_tokens/test/set_token_subscription_state_change_to_core_inactive.txt",
                "tests/data/set_tokens/set_token_subscription_state_change_to_core_inactive.json",
            ),
        ];
        let invalid = vec![(
            "tests/data/set_tokens/test/set_token_delete_user_invalid.txt",
            "tests/data/set_tokens/set_token_delete_user.json",
        )];
        println!();
        for (set_token, json_file) in proper {
            let token = fs::read_to_string(set_token).unwrap();
            let generated = token_from_file(json_file);
            assert_eq!(token, generated);
        }

        for (set_token, json_file) in invalid {
            let token = fs::read_to_string(set_token).unwrap();
            let generated = invalid_token_from_json_file(json_file);
            assert_eq!(token, generated);
        }
    }
}
