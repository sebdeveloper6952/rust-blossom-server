pub struct GetBlob {
    pub pubkey: String,
    pub hash: String,
    pub r#type: String,
    pub size: i64,
    pub created: i64,
    pub blob: Vec<u8>,
}
