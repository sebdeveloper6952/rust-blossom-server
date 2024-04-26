use crate::api::GetBlob;
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

impl From<GetBlob> for BlobDescriptor {
    fn from(blob: GetBlob) -> Self {
        Self {
            url: String::from(""),
            pubkey: blob.pubkey,
            hash: blob.hash,
            r#type: blob.r#type,
            size: blob.size,
            created: blob.created,
        }
    }
}
