use serde::{Deserialize, Serialize};

/// Message from the server to the client.
#[derive(Serialize, Deserialize)]
pub struct ServerMessage {
    pub id: usize,
    pub text: String,
}

/// Message from the client to the server.
#[derive(Serialize, Deserialize)]
pub struct ClientMessage {
    pub text: String,
}

/// Message from the client to the server.
#[derive(Serialize, Deserialize)]
pub struct ClientMessageGQLInit {
    pub r#type: String,
    pub payload: PayloadEmp,
}

#[derive(Serialize, Deserialize)]
pub struct PayloadEmp {}

/// Message from the client to the server.
#[derive(Serialize, Deserialize)]
pub struct ClientMessageGQLPay {
    pub id: String,
    pub r#type: String,
    pub payload: Payload,
}

#[derive(Serialize, Deserialize)]
pub struct Payload {
    pub query: String,
}
