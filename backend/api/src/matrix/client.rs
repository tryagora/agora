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
    #[serde(rename = "home_server")]
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
