use serde::Serialize;

#[derive(Serialize)]
pub struct BlobDescriptor {
    pub pubkey: String,
    pub hash: String,
    pub url: String,
    pub r#type: String,
    pub size: i64,
    pub created: i64,
}
