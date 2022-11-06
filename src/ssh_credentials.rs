use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SshCredentials {
    pub user: String,
    pub id_file: String,
    pub host: String,
}
