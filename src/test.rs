use crate::utils::Client;

pub fn test_email() -> String {
    std::env::var("TEST_EMAIL").expect("Please pass TEST_EMAIL")
}
pub fn test_password() -> String {
    std::env::var("TEST_PASSWORD").expect("Please pass TEST_PASSWORD")
}

pub fn test_client() -> Client {
    Client::from_email_password(&test_email(), &test_password())
}
