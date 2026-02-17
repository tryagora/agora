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

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthFlow {
    pub stages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UiaResponse {
    pub flows: Vec<AuthFlow>,
    pub params: Option<serde_json::Value>,
    pub session: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegistrationRequest {
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthData {
    #[serde(rename = "type")]
    pub auth_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegistrationResponse {
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "access_token")]
    pub access_token: String,
    pub home_server: Option<String>,
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    #[serde(rename = "type")]
    pub login_type: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "access_token")]
    pub access_token: String,
    #[serde(rename = "home_server")]
    pub home_server: Option<String>,
    pub device_id: Option<String>,
}

// Sync types
#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    #[serde(rename = "next_batch")]
    pub next_batch: String,
    pub rooms: Option<Rooms>,
}

#[derive(Debug, Deserialize)]
pub struct Rooms {
    pub join: Option<std::collections::HashMap<String, JoinedRoom>>,
}

#[derive(Debug, Deserialize)]
pub struct JoinedRoom {
    pub timeline: Option<Timeline>,
}

#[derive(Debug, Deserialize)]
pub struct Timeline {
    pub events: Vec<Event>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub sender: String,
    pub content: serde_json::Value,
    #[serde(rename = "event_id")]
    pub event_id: Option<String>,
    pub origin_server_ts: Option<i64>,
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

    pub async fn register(
        &self,
        username: String,
        password: String,
    ) -> Result<RegistrationResponse, MatrixError> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/register?kind=user",
            self.homeserver_url
        );
        
        // Step 1: Get UIA session
        tracing::info!("getting uia session from conduit");
        let uia_response = client
            .post(&url)
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await?;
        
        let uia_status = uia_response.status();
        let uia_text = uia_response.text().await?;
        tracing::info!("uia response status: {}, body: {}", uia_status, uia_text);
        
        let uia: UiaResponse = serde_json::from_str(&uia_text)
            .map_err(|e| MatrixError::ApiError(format!("failed to parse uia response: {}", e)))?;
        
        let session = uia.session.ok_or(MatrixError::NoSession)?;
        tracing::info!("got uia session: {}", session);
        
        // Step 2: Complete registration with auth
        let body = RegistrationRequest {
            username,
            password,
            auth: Some(AuthData {
                auth_type: "m.login.dummy".to_string(),
                session: Some(session),
            }),
        };

        tracing::info!("sending registration request with auth");
        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await?;
        
        let status = response.status();
        let response_text = response.text().await?;
        tracing::info!("registration response status: {}, body: {}", status, response_text);
        
        if status.is_success() {
            let reg_response = serde_json::from_str(&response_text)
                .map_err(|e| MatrixError::ApiError(format!("failed to parse registration response: {}", e)))?;
            Ok(reg_response)
        } else {
            Err(MatrixError::ApiError(response_text))
        }
    }

    pub async fn login(
        &self,
        user: String,
        password: String,
    ) -> Result<LoginResponse, reqwest::Error> {
        let client = reqwest::Client::new();
        let url = format!("{}/_matrix/client/r0/login", self.homeserver_url);
        
        let body = LoginRequest {
            login_type: "m.login.password".to_string(),
            user,
            password,
        };

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await?;
        
        let login_response = response.json::<LoginResponse>().await?;
        Ok(login_response)
    }

    pub async fn sync(
        &self,
        since: Option<String>,
    ) -> Result<SyncResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let mut url = format!("{}/_matrix/client/r0/sync", self.homeserver_url);
        
        // add query parameters
        url.push_str("?timeout=30000");
        if let Some(s) = since {
            url.push_str(&format!("&since={}", s));
        }
        
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            let sync_response = response.json::<SyncResponse>().await?;
            Ok(sync_response)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn send_message(
        &self,
        room_id: String,
        message: String,
    ) -> Result<serde_json::Value, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let txn_id = uuid::Uuid::new_v4().to_string();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/send/m.room.message/{}",
            self.homeserver_url,
            urlencoding::encode(&room_id),
            txn_id
        );
        
        let body = serde_json::json!({
            "msgtype": "m.text",
            "body": message
        });

        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<serde_json::Value>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }
}

#[derive(Debug)]
pub enum MatrixError {
    Reqwest(reqwest::Error),
    NoSession,
    ApiError(String),
    JsonError(serde_json::Error),
}

impl From<reqwest::Error> for MatrixError {
    fn from(err: reqwest::Error) -> Self {
        MatrixError::Reqwest(err)
    }
}

impl From<serde_json::Error> for MatrixError {
    fn from(err: serde_json::Error) -> Self {
        MatrixError::JsonError(err)
    }
}

impl std::fmt::Display for MatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatrixError::Reqwest(e) => write!(f, "request error: {}", e),
            MatrixError::NoSession => write!(f, "no uia session returned"),
            MatrixError::ApiError(e) => write!(f, "api error: {}", e),
            MatrixError::JsonError(e) => write!(f, "json error: {}", e),
        }
    }
}

impl std::error::Error for MatrixError {}
