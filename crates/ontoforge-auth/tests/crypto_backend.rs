//! The JWT crypto backend is selected (aws-lc-rs): sign and verify do not panic.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: u64,
}

#[test]
fn hs256_round_trip_does_not_panic() {
    let secret = [7_u8; 32];
    let claims = Claims {
        sub: "user-1".to_owned(),
        exp: 4_102_444_800, // 2100-01-01
    };
    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
    .expect("encode");
    let decoded = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(&secret),
        &Validation::new(Algorithm::HS256),
    )
    .expect("decode");
    assert_eq!(decoded.claims.sub, "user-1");
}
