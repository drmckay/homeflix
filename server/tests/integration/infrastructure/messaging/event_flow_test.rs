//! Integration tests for Event Flow
//!
//! Tests cover:
//! - Complete event flow from publication to handler execution
//! - Event sourcing integration
//! - Multiple handlers processing same event
//! - Event persistence

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use sqlx::SqlitePool;
use homeflixd::infrastructure::messaging::{InMemoryEventBus, PersistentEventBus};
use homeflixd::infrastructure::event_sourcing::{EventStore, SqliteEventPersistence};
use homeflixd::domain::events::{
    SubtitleGenerationStartedEvent,
    SubtitleGenerationCompletedEvent,
    ProgressUpdatedEvent,
};
use homeflixd::interfaces::messaging::{EventBus, EventHandler, DomainEvent};
use homeflixd::shared::error::MessagingError;

/// Test handler that tracks received events
struct TestHandler {
    received_count: Arc<AtomicU32>,
    last_media_id: Arc<std::sync::Mutex<Option<i64>>>,
}

#[async_trait::async_trait]
impl EventHandler<SubtitleGenerationStartedEvent> for TestHandler {
    async fn handle(&self, event: SubtitleGenerationStartedEvent) -> Result<(), MessagingError> {
        self.received_count.fetch_add(1, Ordering::Relaxed);
        *self.last_media_id.lock().unwrap() = Some(event.media_id);
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<ProgressUpdatedEvent> for TestHandler {
    async fn handle(&self, event: ProgressUpdatedEvent) -> Result<(), MessagingError> {
        self.received_count.fetch_add(1, Ordering::Relaxed);
        *self.last_media_id.lock().unwrap() = Some(event.media_id);
        Ok(())
    }
}

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create test database");
    
    // Initialize schema
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            aggregate_id TEXT,
            aggregate_type TEXT,
            payload TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            correlation_id TEXT,
            causation_id TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
        CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events(aggregate_type, aggregate_id);
        CREATE INDEX IF NOT EXISTS idx_events_created ON events(created_at);
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to create events table");
    
    pool
}

#[tokio::test]
async fn test_event_flow_with_persistence() {
    let pool = setup_test_db().await;
    let event_persistence = Arc::new(SqliteEventPersistence::new(Arc::new(pool.clone())));
    let event_store = Arc::new(EventStore::new(event_persistence));
    
    let inner_bus = Arc::new(InMemoryEventBus::new());
    let persistent_bus = Arc::new(PersistentEventBus::new(inner_bus.clone(), event_store));
    
    let received_count = Arc::new(AtomicU32::new(0));
    let last_media_id = Arc::new(std::sync::Mutex::new(None));
    
    let handler = Arc::new(TestHandler {
        received_count: received_count.clone(),
        last_media_id: last_media_id.clone(),
    });
    
    inner_bus.subscribe::<SubtitleGenerationStartedEvent>(handler).await.unwrap();
    
    let event = SubtitleGenerationStartedEvent::new(
        42,
        "test.srt".to_string(),
        "en".to_string(),
        false,
        "test-job".to_string(),
    );

    // Publish through persistent bus
    persistent_bus.publish(event).await.unwrap();
    
    // Give handler time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    
    // Verify handler was called
    assert_eq!(received_count.load(Ordering::Relaxed), 1);
    assert_eq!(*last_media_id.lock().unwrap(), Some(42));
    
    // Verify event was persisted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM events WHERE event_type = 'SubtitleGenerationStartedEvent'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn test_multiple_handlers_integration() {
    let event_bus = InMemoryEventBus::new();
    
    let count1 = Arc::new(AtomicU32::new(0));
    let count2 = Arc::new(AtomicU32::new(0));
    
    let handler1 = Arc::new(TestHandler {
        received_count: count1.clone(),
        last_media_id: Arc::new(std::sync::Mutex::new(None)),
    });
    let handler2 = Arc::new(TestHandler {
        received_count: count2.clone(),
        last_media_id: Arc::new(std::sync::Mutex::new(None)),
    });
    
    event_bus.subscribe::<ProgressUpdatedEvent>(handler1).await.unwrap();
    event_bus.subscribe::<ProgressUpdatedEvent>(handler2).await.unwrap();
    
    let event = ProgressUpdatedEvent::new(
        100,
        3600,
        true,
    );

    event_bus.publish(event).await.unwrap();
    
    // Give handlers time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    
    // Both handlers should have received the event
    assert_eq!(count1.load(Ordering::Relaxed), 1);
    assert_eq!(count2.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_event_ordering() {
    let event_bus = InMemoryEventBus::new();
    let received_events = Arc::new(std::sync::Mutex::new(Vec::new()));
    
    struct OrderingHandler {
        received: Arc<std::sync::Mutex<Vec<i64>>>,
    }
    
    #[async_trait::async_trait]
    impl EventHandler<SubtitleGenerationStartedEvent> for OrderingHandler {
        async fn handle(&self, event: SubtitleGenerationStartedEvent) -> Result<(), MessagingError> {
            self.received.lock().unwrap().push(event.media_id);
            Ok(())
        }
    }
    
    let handler = Arc::new(OrderingHandler {
        received: received_events.clone(),
    });
    
    event_bus.subscribe::<SubtitleGenerationStartedEvent>(handler).await.unwrap();
    
    // Publish events in sequence
    for i in 1..=5 {
        let event = SubtitleGenerationStartedEvent::new(
            i,
            format!("test{}.srt", i),
            "en".to_string(),
            false,
            format!("job-{}", i),
        );
        event_bus.publish(event).await.unwrap();
    }
    
    // Give handlers time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Verify events were received in order
    let received = received_events.lock().unwrap();
    assert_eq!(received.len(), 5);
    assert_eq!(received, &vec![1, 2, 3, 4, 5]);
}
