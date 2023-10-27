use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct File {
    pub file_path: String,
    pub file_name: String,
    pub size: u64,
    pub encoding: String,
    pub content: String,
    pub content_sha256: String,
    #[serde(rename = "ref")]
    pub _ref: String,
    pub blob_id: String,
    pub commit_id: String,
    pub last_commit_id: String,
    pub execute_filemode: bool,
}

impl File {
    pub fn get_content(&self) -> String {
        let decoded: Vec<u8> = general_purpose::STANDARD.decode(&self.content).unwrap();
        String::from_utf8(decoded).unwrap()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    pub fn deserialization_test() {
        let json = r#"
        {
            "file_name": "key.rb",
            "file_path": "app/models/key.rb",
            "size": 1476,
            "encoding": "base64",
            "content": "SGVsbG8gV29ybGQ=",
            "content_sha256": "4c294617b60715c1d218e61164a3abd4808a4284cbc30e6728a01ad9aada4481",
            "ref": "main",
            "blob_id": "79f7bbd25901e8334750839545a9bd021f0e4c83",
            "commit_id": "d5a3ff139356ce33e37e73add446f16869741b50",
            "last_commit_id": "570e7b2abdd848b95f2f578043fc23bd6f6fd24d",
            "execute_filemode": false
         }"#;
        let file: super::File = serde_json::from_str(json).unwrap();
        assert_eq!(file.file_name, "key.rb");
        assert_eq!(file.get_content(), "Hello World");
    }
}
