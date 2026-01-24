//! Unit tests for Event Bus
//!
//! Tests cover:
//! - Event publishing
//! - Event subscription
//! - Handler invocation
//! - Multiple handlers per event type

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use homeflixd::infrastructure::messaging::InMemoryEventBus;
use homeflixd::domain::events::{
    SubtitleGenerationStartedEvent,
    SubtitleGenerationCompletedEvent,
    ProgressUpdatedEvent,
};
use homeflixd::interfaces::messaging::{EventBus, EventHandler, DomainEvent};
use homeflixd::shared::error::MessagingError;

/// Test handler that counts invocations
struct CountingHandler {
    count: Arc<AtomicU32>,
}

#[async_trait::async_trait]
impl EventHandler<SubtitleGenerationStartedEvent> for CountingHandler {
    async fn handle(&self, _event: SubtitleGenerationStartedEvent) -> Result<(), MessagingError> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventHandler<SubtitleGenerationCompletedEvent> for CountingHandler {
    async fn handle(&self, _event: SubtitleGenerationCompletedEvent) -> Result<(), MessagingError> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[tokio::test]
async fn test_event_publish_without_subscribers() {
    let event_bus = InMemoryEventBus::new();
    
    let event = SubtitleGenerationStartedEvent::new(
        1,
        "test.srt".to_string(),
        "en".to_string(),
        false,
        "test-job".to_string(),
    );

    // Should not panic even without subscribers
    let result = event_bus.publish(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_event_publish_with_subscriber() {
    let event_bus = InMemoryEventBus::new();
    let count = Arc::new(AtomicU32::new(0));
    
    let handler = Arc::new(CountingHandler {
        count: count.clone(),
    });
    
    event_bus.subscribe::<SubtitleGenerationStartedEvent>(handler).await.unwrap();
    
    let event = SubtitleGenerationStartedEvent::new(
        1,
        "test.srt".to_string(),
        "en".to_string(),
        false,
        "test-job".to_string(),
    );

    event_bus.publish(event).await.unwrap();
    
    // Give handler time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    assert_eq!(count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_multiple_handlers_same_event() {
    let event_bus = InMemoryEventBus::new();
    let count1 = Arc::new(AtomicU32::new(0));
    let count2 = Arc::new(AtomicU32::new(0));
    
    let handler1 = Arc::new(CountingHandler {
        count: count1.clone(),
    });
    let handler2 = Arc::new(CountingHandler {
        count: count2.clone(),
    });
    
    event_bus.subscribe::<SubtitleGenerationStartedEvent>(handler1).await.unwrap();
    event_bus.subscribe::<SubtitleGenerationStartedEvent>(handler2).await.unwrap();
    
    let event = SubtitleGenerationStartedEvent::new(
        1,
        "test.srt".to_string(),
        "en".to_string(),
        false,
        "test-job".to_string(),
    );

    event_bus.publish(event).await.unwrap();
    
    // Give handlers time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    assert_eq!(count1.load(Ordering::Relaxed), 1);
    assert_eq!(count2.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_different_event_types() {
    let event_bus = InMemoryEventBus::new();
    let started_count = Arc::new(AtomicU32::new(0));
    let completed_count = Arc::new(AtomicU32::new(0));
    
    let started_handler = Arc::new(CountingHandler {
        count: started_count.clone(),
    });
    let completed_handler = Arc::new(CountingHandler {
        count: completed_count.clone(),
    });
    
    event_bus.subscribe::<SubtitleGenerationStartedEvent>(started_handler).await.unwrap();
    event_bus.subscribe::<SubtitleGenerationCompletedEvent>(completed_handler).await.unwrap();
    
    let started_event = SubtitleGenerationStartedEvent::new(
        1,
        "test.srt".to_string(),
        "en".to_string(),
        false,
        "test-job".to_string(),
    );
    let completed_event = SubtitleGenerationCompletedEvent::new(
        1,
        "test-job".to_string(),
        "test.srt".to_string(),
        "en".to_string(),
        false,
    );

    event_bus.publish(started_event).await.unwrap();
    event_bus.publish(completed_event).await.unwrap();
    
    // Give handlers time to execute
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    assert_eq!(started_count.load(Ordering::Relaxed), 1);
    assert_eq!(completed_count.load(Ordering::Relaxed), 1);
}
