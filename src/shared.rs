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


pub struct Clock {
    hours: i64,
    minutes: i64,
    seconds: i64,
}

impl Clock {

    ///
    /// Create a new clock.
    ///
    pub fn new() -> Clock {
        Clock {
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }

    ///
    /// Set the clock time in milliseconds.
    ///
    /// * `ms` - Milliseconds to set time from.
    pub fn set_time_ms(&mut self, ms: i64) {
        self.seconds = (ms / 1000) % 60;
        self.minutes = (ms / (1000 * 60)) % 60;
        self.hours = (ms / (1000 * 60 * 60)) % 24;
    }

    ///
    /// Set the clock time in seconds.
    ///
    /// * `seconds` - Seconds to set time from.
    pub fn set_time_secs(&mut self, seconds: i64) {
        self.set_time_ms(seconds * 1000);
    }

    ///
    /// Get the clock time in hh:mm:ss notation.
    ///
    pub fn get_time(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hours, self.minutes, self.seconds)
    }
}
