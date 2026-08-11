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

#[cfg(test)]
mod tests {
    use super::*;

    async fn record(buf: &EventBuffer, path: &str) -> u64 {
        buf.record_request_started("GET", path, &[], b"").await
    }

    #[tokio::test]
    async fn ids_are_unique_and_monotonic() {
        let buf = EventBuffer::new(10);

        let first = record(&buf, "/a").await;
        let second = record(&buf, "/b").await;

        assert!(second > first, "correlation depends on ids never repeating");
    }

    #[tokio::test]
    async fn a_completed_response_is_attached_to_its_own_request() {
        let buf = EventBuffer::new(10);
        let first = record(&buf, "/first").await;
        let second = record(&buf, "/second").await;

        buf.record_request_completed(second, 404, &[], b"nope", 12.5)
            .await;

        let (events, _) = buf.snapshot().await;
        let first_event = events.iter().find(|e| e.id == first).unwrap();
        let second_event = events.iter().find(|e| e.id == second).unwrap();

        assert!(
            first_event.response.is_none(),
            "an unrelated request must stay pending"
        );
        assert_eq!(second_event.response.as_ref().unwrap().status, 404);
        assert_eq!(second_event.duration_ms, Some(12.5));
    }

    #[tokio::test]
    async fn completing_an_unknown_id_is_ignored_rather_than_panicking() {
        let buf = EventBuffer::new(10);
        record(&buf, "/a").await;

        // Happens when a response arrives after its event was evicted.
        buf.record_request_completed(9999, 200, &[], b"", 1.0).await;

        let (events, _) = buf.snapshot().await;
        assert_eq!(events.len(), 1);
        assert!(events[0].response.is_none());
    }

    #[tokio::test]
    async fn the_buffer_evicts_oldest_first_and_never_exceeds_its_cap() {
        let buf = EventBuffer::new(3);

        for i in 0..5 {
            record(&buf, &format!("/{i}")).await;
        }

        let (events, _) = buf.snapshot().await;
        assert_eq!(events.len(), 3, "cap must hold under sustained load");
        assert_eq!(
            events.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![3, 4, 5],
            "the three most recent requests must survive"
        );
    }

    #[tokio::test]
    async fn clear_empties_the_buffer_without_reusing_ids() {
        let buf = EventBuffer::new(10);
        let before = record(&buf, "/a").await;

        buf.clear().await;
        assert!(buf.snapshot().await.0.is_empty());

        let after = record(&buf, "/b").await;
        assert!(after > before, "ids must not restart after a clear");
    }

    #[tokio::test]
    async fn subscribers_receive_start_and_completion_in_order() {
        let buf = EventBuffer::new(10);
        let mut rx = buf.subscribe();

        let id = record(&buf, "/watched").await;
        buf.record_request_completed(id, 200, &[], b"ok", 3.0).await;

        match rx.recv().await.unwrap() {
            WsMessage::RequestStarted { event } => assert_eq!(event.id, id),
            other => panic!("expected RequestStarted, got {other:?}"),
        }
        match rx.recv().await.unwrap() {
            WsMessage::RequestCompleted {
                id: completed,
                response,
                ..
            } => {
                assert_eq!(completed, id);
                assert_eq!(response.status, 200);
            }
            other => panic!("expected RequestCompleted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn recording_without_subscribers_does_not_fail() {
        // `tx.send` errors when nobody is listening; that must stay non-fatal.
        let buf = EventBuffer::new(10);
        let id = record(&buf, "/a").await;
        buf.record_request_completed(id, 200, &[], b"", 1.0).await;

        assert_eq!(buf.snapshot().await.0.len(), 1);
    }

    #[test]
    fn the_timestamp_is_iso8601_shaped_at_a_known_date() {
        // 2026-08-11 as days since epoch, guarding the hand-rolled calendar math.
        assert_eq!(days_to_ymd(20676), (2026, 8, 11));
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // Leap day: the civil-from-days algorithm must not drift.
        assert_eq!(days_to_ymd(59), (1970, 3, 1));
        assert_eq!(days_to_ymd(11016), (2000, 2, 29));
    }

    #[test]
    fn the_formatted_timestamp_has_the_expected_layout() {
        let ts = chrono_iso8601_now();

        assert_eq!(ts.len(), 24, "YYYY-MM-DDTHH:MM:SS.mmmZ — got {ts}");
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], "T");
    }
}
