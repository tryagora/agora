use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct MatrixClient {
    pub homeserver_url: String,
    pub access_token: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatrixVersions {
    pub versions: Vec<String>,
}

impl MatrixClient {
    pub fn new(homeserver_url: String) -> Self {
        Self {
            homeserver_url,
            access_token: None,
            user_id: None,
        }
    }

    pub fn with_auth(homeserver_url: String, access_token: String, user_id: String) -> Self {
        Self {
            homeserver_url,
            access_token: Some(access_token),
            user_id: Some(user_id),
        }
    }

    pub async fn get_versions(&self) -> Result<MatrixVersions, reqwest::Error> {
        let client = reqwest::Client::new();
        let url = format!("{}/_matrix/client/versions", self.homeserver_url);
        let response = client.get(&url).send().await?;
        let versions = response.json::<MatrixVersions>().await?;
        Ok(versions)
    }
}
