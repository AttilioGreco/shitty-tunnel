use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::sync::{broadcast, RwLock};

use super::event::{RequestData, RequestEvent, ResponseData};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "snapshot")]
    Snapshot { events: Vec<RequestEvent>, epoch_ms: f64 },
    #[serde(rename = "request_started")]
    RequestStarted { event: RequestEvent },
    #[serde(rename = "request_completed")]
    RequestCompleted { id: u64, response: ResponseData, duration_ms: f64 },
    #[serde(rename = "cleared")]
    Cleared,
}

pub struct EventBuffer {
    events: RwLock<VecDeque<RequestEvent>>,
    max_events: usize,
    next_id: AtomicU64,
    epoch: Instant,
    tx: broadcast::Sender<WsMessage>,
}

impl EventBuffer {
    pub fn new(max_events: usize) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            events: RwLock::new(VecDeque::with_capacity(max_events)),
            max_events,
            next_id: AtomicU64::new(1),
            epoch: Instant::now(),
            tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsMessage> {
        self.tx.subscribe()
    }

    pub async fn record_request_started(
        &self,
        method: &str,
        path: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let offset_ms = self.epoch.elapsed().as_secs_f64() * 1000.0;
        let timestamp = chrono_iso8601_now();

        let event = RequestEvent {
            id,
            timestamp,
            offset_ms,
            request: RequestData::from_parts(method, path, headers, body),
            response: None,
            duration_ms: None,
        };

        {
            let mut events = self.events.write().await;
            if events.len() >= self.max_events {
                events.pop_front();
            }
            events.push_back(event.clone());
        }

        let _ = self.tx.send(WsMessage::RequestStarted { event });
        id
    }

    pub async fn record_request_completed(
        &self,
        id: u64,
        status: u16,
        headers: &[(String, String)],
        body: &[u8],
        duration_ms: f64,
    ) {
        let response = ResponseData::from_parts(status, headers, body);

        {
            let mut events = self.events.write().await;
            if let Some(event) = events.iter_mut().find(|e| e.id == id) {
                event.response = Some(response.clone());
                event.duration_ms = Some(duration_ms);
            }
        }

        let _ = self.tx.send(WsMessage::RequestCompleted {
            id,
            response,
            duration_ms,
        });
    }

    pub async fn snapshot(&self) -> (Vec<RequestEvent>, f64) {
        let events = self.events.read().await;
        let epoch_ms = self.epoch.elapsed().as_secs_f64() * 1000.0;
        (events.iter().cloned().collect(), epoch_ms)
    }

    pub async fn clear(&self) {
        self.events.write().await.clear();
        let _ = self.tx.send(WsMessage::Cleared);
    }
}

fn chrono_iso8601_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    // Simple ISO-8601-ish format without pulling in chrono
    let total_mins = secs / 60;
    let s = secs % 60;
    let total_hours = total_mins / 60;
    let m = total_mins % 60;
    let total_days = total_hours / 24;
    let h = total_hours % 24;

    // Days since epoch to y-m-d (simplified)
    let (y, mo, d) = days_to_ymd(total_days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    days += 719468;
    let era = days / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
