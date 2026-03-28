use uuid::Uuid;

#[test]
fn system_user_serialization_skips_password() {
    use storm_api::models::user::UserInfo;

    let info = UserInfo {
        id: Uuid::new_v4(),
        name: "Alice".to_string(),
        email: Some("alice@test.com".to_string()),
        username: "alice".to_string(),
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["username"], "alice");
    assert_eq!(json["email"], "alice@test.com");
}
