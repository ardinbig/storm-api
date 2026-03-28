use storm_api::utils::password;

#[test]
fn hash_valid_password_returns_ok() {
    let result = password::hash("secure.password");
    assert!(result.is_ok());
    let hashed = result.unwrap();
    assert!(hashed.starts_with("$argon2"));
}

#[test]
fn hash_empty_password_returns_ok() {
    let result = password::hash("");
    assert!(result.is_ok());
}

#[test]
fn hash_produces_different_hashes_for_same_input() {
    let h1 = password::hash("same.pass").unwrap();
    let h2 = password::hash("same.pass").unwrap();
    assert_ne!(h1, h2, "Different salts should produce different hashes");
}
