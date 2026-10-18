use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u16 = 1;
pub const EVENT_STREAM: &str = "ctc:events:v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentState {
    Idle,
    Analyzing,
    Meeting,
    Approving,
    Executing,
    Monitoring,
    Blocked,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    pub schema_version: u16,
    pub event_id: Uuid,
    pub correlation_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub producer: String,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventPayload {
    ServiceStarted { service: String, version: String },
}

impl EventEnvelope {
    pub fn service_started(service: &str, version: &str) -> Self {
        let id = Uuid::new_v4();
        Self {
            schema_version: SCHEMA_VERSION,
            event_id: id,
            correlation_id: id,
            occurred_at: Utc::now(),
            producer: service.to_owned(),
            payload: EventPayload::ServiceStarted {
                service: service.to_owned(),
                version: version.to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_round_trip_preserves_identity_and_version() {
        let event = EventEnvelope::service_started("api", "0.1.0");
        let encoded = serde_json::to_string(&event).unwrap();
        let decoded: EventEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(event.event_id, decoded.event_id);
        assert_eq!(decoded.schema_version, 1);
        assert!(encoded.contains("service_started"));
    }

    #[test]
    fn unknown_event_types_are_rejected() {
        assert!(serde_json::from_str::<EventPayload>(r#"{"type":"place_live_order"}"#).is_err());
    }

    #[test]
    fn agent_state_wire_format_is_stable() {
        assert_eq!(
            serde_json::to_string(&AgentState::Blocked).unwrap(),
            "\"BLOCKED\""
        );
    }
}
