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

// encode a matrix identifier for use in url paths
// preserves sigils (!, @) and colon (:) but encodes # to %23 (fragment separator)
fn encode_matrix_id(id: &str) -> String {
    id.replace('%', "%25")
        .replace('#', "%23")
        .replace(' ', "%20")
        .replace('?', "%3F")
        .replace('&', "%26")
        .replace('{', "%7B")
        .replace('}', "%7D")
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

    /// send a message event with arbitrary content — used for call signaling
    pub async fn send_message_content(
        &self,
        room_id: String,
        content: serde_json::Value,
    ) -> Result<serde_json::Value, MatrixError> {
        let token = self.access_token.as_ref().ok_or(MatrixError::NoSession)?;
        let client = reqwest::Client::new();
        let txn_id = uuid::Uuid::new_v4().to_string();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/send/m.room.message/{}",
            self.homeserver_url,
            encode_matrix_id(&room_id),
            txn_id
        );
        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&content)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(response.json::<serde_json::Value>().await?)
        } else {
            Err(MatrixError::ApiError(response.text().await?))
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
            encode_matrix_id(&room_id),
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

    // server/room management
    pub async fn create_room(
        &self,
        name: String,
        topic: Option<String>,
        is_space: bool,
    ) -> Result<CreateRoomResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!("{}/_matrix/client/r0/createRoom", self.homeserver_url);
        
        let mut body = serde_json::json!({
            "name": name,
            "preset": "public_chat",
            "room_version": "9"
        });
        
        if let Some(t) = topic {
            body["topic"] = serde_json::Value::String(t);
        }
        
        if is_space {
            body["creation_content"] = serde_json::json!({
                "type": "m.space"
            });
        }

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<CreateRoomResponse>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    // create a room alias for an existing room
    pub async fn create_room_alias(
        &self,
        room_alias: String,
        room_id: String,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/directory/room/{}",
            self.homeserver_url,
            encode_matrix_id(&room_alias)
        );
        
        let body = serde_json::json!({
            "room_id": room_id
        });

        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn join_room(
        &self,
        room_id_or_alias: String,
    ) -> Result<JoinRoomResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/join/{}",
            self.homeserver_url,
            encode_matrix_id(&room_id_or_alias)
        );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({}))
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<JoinRoomResponse>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn get_joined_rooms(&self) -> Result<JoinedRoomsResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!("{}/_matrix/client/r0/joined_rooms", self.homeserver_url);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<JoinedRoomsResponse>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn get_room_members(
        &self,
        room_id: String,
    ) -> Result<RoomMembersResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/members",
            self.homeserver_url,
            encode_matrix_id(&room_id)
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<RoomMembersResponse>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn get_room_state(
        &self,
        room_id: String,
    ) -> Result<Vec<RoomStateEvent>, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/state",
            self.homeserver_url,
            encode_matrix_id(&room_id)
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<Vec<RoomStateEvent>>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    // add a room as a child of a space (m.space.child state event)
    pub async fn add_space_child(
        &self,
        space_id: String,
        child_room_id: String,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/state/m.space.child/{}",
            self.homeserver_url,
            encode_matrix_id(&space_id),
            encode_matrix_id(&child_room_id)
        );
        
        let body = serde_json::json!({
            "via": ["localhost"]
        });

        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    // remove a room as a child of a space (delete m.space.child state event)
    pub async fn remove_space_child(
        &self,
        space_id: String,
        child_room_id: String,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/state/m.space.child/{}",
            self.homeserver_url,
            encode_matrix_id(&space_id),
            encode_matrix_id(&child_room_id)
        );

        let response = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn invite_user(
        &self,
        room_id: String,
        user_id: String,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/invite",
            self.homeserver_url,
            encode_matrix_id(&room_id)
        );
        
        let body = serde_json::json!({
            "user_id": user_id
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn leave_room(
        &self,
        room_id: String,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/leave",
            self.homeserver_url,
            encode_matrix_id(&room_id)
        );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({}))
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn forget_room(
        &self,
        room_id: String,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/forget",
            self.homeserver_url,
            encode_matrix_id(&room_id)
        );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({}))
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn get_power_levels(
        &self,
        room_id: String,
    ) -> Result<PowerLevelsResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/state/m.room.power_levels",
            self.homeserver_url,
            encode_matrix_id(&room_id)
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<PowerLevelsResponse>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    pub async fn set_power_levels(
        &self,
        room_id: String,
        power_levels: PowerLevelsRequest,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/state/m.room.power_levels",
            self.homeserver_url,
            encode_matrix_id(&room_id)
        );

        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&power_levels)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    // ── presence ──────────────────────────────────────────────────────────────

    pub async fn set_presence(
        &self,
        user_id: String,
        presence: String,
        status_msg: Option<String>,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref().ok_or(MatrixError::NoSession)?;
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/presence/{}/status",
            self.homeserver_url,
            encode_matrix_id(&user_id)
        );
        let mut body = serde_json::json!({ "presence": presence });
        if let Some(msg) = status_msg {
            body["status_msg"] = serde_json::Value::String(msg);
        }
        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await?;
            Err(MatrixError::ApiError(err))
        }
    }

    pub async fn get_presence(
        &self,
        user_id: String,
    ) -> Result<PresenceData, MatrixError> {
        let token = self.access_token.as_ref().ok_or(MatrixError::NoSession)?;
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/presence/{}/status",
            self.homeserver_url,
            encode_matrix_id(&user_id)
        );
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        if response.status().is_success() {
            let data = response.json::<PresenceData>().await?;
            Ok(data)
        } else {
            let err = response.text().await?;
            Err(MatrixError::ApiError(err))
        }
    }

    // ── profile ───────────────────────────────────────────────────────────────

    pub async fn get_profile(
        &self,
        user_id: String,
    ) -> Result<ProfileData, MatrixError> {
        let token = self.access_token.as_ref().ok_or(MatrixError::NoSession)?;
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/profile/{}",
            self.homeserver_url,
            encode_matrix_id(&user_id)
        );
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;
        if response.status().is_success() {
            let data = response.json::<ProfileData>().await?;
            Ok(data)
        } else {
            let err = response.text().await?;
            Err(MatrixError::ApiError(err))
        }
    }

    pub async fn set_displayname(
        &self,
        user_id: String,
        displayname: String,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref().ok_or(MatrixError::NoSession)?;
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/profile/{}/displayname",
            self.homeserver_url,
            encode_matrix_id(&user_id)
        );
        let body = serde_json::json!({ "displayname": displayname });
        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await?;
            Err(MatrixError::ApiError(err))
        }
    }

    /// create a direct message room (m.direct) with the given user.
    /// `display_name` is used as the room name so the DM list can show it.
    pub async fn create_dm_room(
        &self,
        other_user_id: String,
        display_name: String,
    ) -> Result<CreateRoomResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;

        let client = reqwest::Client::new();
        let url = format!("{}/_matrix/client/r0/createRoom", self.homeserver_url);

        let body = serde_json::json!({
            "name": display_name,
            "preset": "trusted_private_chat",
            "is_direct": true,
            "invite": [other_user_id]
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            let result = response.json::<CreateRoomResponse>().await?;
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }

    /// send a state event to a room (PUT /rooms/{room_id}/state/{event_type}/{state_key})
    pub async fn send_state_event(
        &self,
        room_id: String,
        event_type: String,
        state_key: String,
        content: serde_json::Value,
    ) -> Result<(), MatrixError> {
        let token = self.access_token.as_ref().ok_or(MatrixError::NoSession)?;
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/state/{}/{}",
            self.homeserver_url,
            encode_matrix_id(&room_id),
            event_type,
            state_key
        );
        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&content)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await?;
            Err(MatrixError::ApiError(err))
        }
    }

    pub async fn create_category(
        &self,
        name: String,
        parent_space_id: String,
    ) -> Result<CreateRoomResponse, MatrixError> {
        let token = self.access_token.as_ref()
            .ok_or(MatrixError::NoSession)?;
        
        let client = reqwest::Client::new();
        let url = format!("{}/_matrix/client/r0/createRoom", self.homeserver_url);
        
        let body = serde_json::json!({
            "name": name,
            "preset": "public_chat",
            "room_version": "9",
            "creation_content": {
                "type": "m.space"
            }
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;
        
        if response.status().is_success() {
            let result = response.json::<CreateRoomResponse>().await?;
            
            // Add the new category (subspace) as a child of the parent space
            if let Err(e) = self.add_space_child(parent_space_id, result.room_id.clone()).await {
                tracing::warn!("failed to add category to parent space: {}", e);
            }
            
            Ok(result)
        } else {
            let error_text = response.text().await?;
            Err(MatrixError::ApiError(error_text))
        }
    }
}

// room/server response types
#[derive(Debug, Deserialize)]
pub struct CreateRoomResponse {
    #[serde(rename = "room_id")]
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinRoomResponse {
    #[serde(rename = "room_id")]
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct JoinedRoomsResponse {
    #[serde(rename = "joined_rooms")]
    pub joined_rooms: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoomMembersResponse {
    #[serde(rename = "chunk")]
    pub members: Vec<RoomMemberEvent>,
}

#[derive(Debug, Deserialize)]
pub struct RoomMemberEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub sender: String,
    #[serde(rename = "state_key")]
    pub state_key: String,
    pub content: RoomMemberContent,
}

#[derive(Debug, Deserialize)]
pub struct RoomMemberContent {
    #[serde(rename = "displayname")]
    pub display_name: Option<String>,
    #[serde(rename = "avatar_url")]
    pub avatar_url: Option<String>,
    pub membership: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoomStateEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub state_key: Option<String>,
    pub content: serde_json::Value,
    pub sender: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PowerLevelsRequest {
    pub users: std::collections::HashMap<String, i64>,
    pub users_default: Option<i64>,
    pub events: Option<std::collections::HashMap<String, i64>>,
    pub events_default: Option<i64>,
    pub state_default: Option<i64>,
    pub ban: Option<i64>,
    pub kick: Option<i64>,
    pub redact: Option<i64>,
    pub invite: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PowerLevelsResponse {
    pub users: Option<std::collections::HashMap<String, i64>>,
    pub users_default: Option<i64>,
    pub events: Option<std::collections::HashMap<String, i64>>,
    pub events_default: Option<i64>,
    pub state_default: Option<i64>,
    pub ban: Option<i64>,
    pub kick: Option<i64>,
    pub redact: Option<i64>,
    pub invite: Option<i64>,
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

#[derive(Debug, Deserialize)]
pub struct PresenceData {
    pub presence: String,
    pub last_active_ago: Option<i64>,
    pub status_msg: Option<String>,
    pub currently_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileData {
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
}
