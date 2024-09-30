use poem_openapi::{auth::ApiKey, SecurityScheme};

#[derive(SecurityScheme)]
#[oai(
    rename = "Main Secret",
    ty = "api_key",
    key_in = "header",
    key_name = "X-API-Key"
)]
pub struct HeaderXApiKey(pub ApiKey);
