//! # Chronos Cognitive Bus
//!
//! This crate provides the central publish/subscribe event transport for the Chronos PCOS.
//! It routes `ChronosEvent` objects between all decoupled subsystems (adapters, engines, UI, persistence).
//!
//! No subsystem is permitted to communicate directly with another subsystem. All context must
//! flow through the `EventBus`. Publishers and Subscribers are completely unaware of each other.

use async_trait::async_trait;
use chronos_core::ChronosEvent;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

/// Error types for the Cognitive Bus.
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("Failed to publish event to the bus: {0}")]
    PublishError(String),
    #[error("Failed to receive event from the bus: {0}")]
    ReceiveError(String),
    #[error("Bus is shutting down or dropped")]
    BusShutdown,
}

/// The core interface for the Cognitive Bus.
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publishes a ChronosEvent to all active subscribers.
    fn publish(&self, event: ChronosEvent) -> Result<usize, BusError>;

    /// Registers a new subscriber, returning a handle to receive events.
    fn subscribe(&self) -> Box<dyn Subscriber>;

    /// Closes the bus and initiates graceful shutdown.
    fn shutdown(&self);
}

/// Interface for entities that publish events to the bus.
/// (Optional convenience trait, though most will just call `bus.publish()`).
#[async_trait]
pub trait Publisher: Send + Sync {
    async fn publish_event(&self, event: ChronosEvent) -> Result<(), BusError>;
}

/// Interface for receiving events from the bus.
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Wait for the next event to arrive on the bus.
    async fn next_event(&mut self) -> Result<ChronosEvent, BusError>;
}

/// A concrete implementation of the EventBus using tokio's broadcast channel.
pub struct MemoryEventBus {
    sender: broadcast::Sender<ChronosEvent>,
}

impl MemoryEventBus {
    /// Creates a new MemoryEventBus with a specified capacity.
    /// The capacity determines how many unread events can queue up before subscribers fall behind.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

#[async_trait]
impl EventBus for MemoryEventBus {
    fn publish(&self, event: ChronosEvent) -> Result<usize, BusError> {
        // broadcast::Sender::send returns the number of active receivers.
        // It returns an error if there are no active receivers, which is a valid state (just 0 delivered).
        match self.sender.send(event) {
            Ok(count) => Ok(count),
            Err(_) => Ok(0), // No subscribers yet, this is fine.
        }
    }

    fn subscribe(&self) -> Box<dyn Subscriber> {
        Box::new(MemorySubscriber {
            receiver: self.sender.subscribe(),
        })
    }

    fn shutdown(&self) {
        // Dropping the receivers/senders happens naturally, but we can artificially drop the channels or signal.
        // In a broadcast channel, if the sender is dropped, receivers get RecvError::Closed.
        // However, since `sender` is held by `MemoryEventBus`, when the bus goes out of scope it closes.
        // To implement explicit shutdown without taking ownership of `self`, we rely on dropping the bus object itself,
        // or we could wrap the sender in an Option/Mutex if explicit mid-lifetime shutdown was needed.
        // For now, dropping the bus is the idiomatic graceful shutdown.
    }
}

pub struct MemorySubscriber {
    receiver: broadcast::Receiver<ChronosEvent>,
}

#[async_trait]
impl Subscriber for MemorySubscriber {
    async fn next_event(&mut self) -> Result<ChronosEvent, BusError> {
        match self.receiver.recv().await {
            Ok(event) => Ok(event),
            Err(RecvError::Closed) => Err(BusError::BusShutdown),
            Err(RecvError::Lagged(skipped)) => {
                // In a production system, lagging means we missed events.
                // For Chronos, we should log this and continue to the next available event.
                Err(BusError::ReceiveError(format!("Subscriber lagged by {} events", skipped)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_core::ChronosEvent;
    use serde_json::json;

    #[tokio::test]
    async fn test_bus_publish_and_subscribe() {
        let bus = MemoryEventBus::new(100);
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        let event = ChronosEvent::new(
            "TestEvent",
            "TestSource",
            json!({"key": "value"}),
        );

        // Publish to bus
        let received_count = bus.publish(event.clone()).expect("Failed to publish");
        assert_eq!(received_count, 2);

        // Verify sub1 received the event
        let received1 = sub1.next_event().await.expect("sub1 failed to receive");
        assert_eq!(received1.id, event.id);

        // Verify sub2 received the exact same event
        let received2 = sub2.next_event().await.expect("sub2 failed to receive");
        assert_eq!(received2.id, event.id);
    }

    #[tokio::test]
    async fn test_bus_no_subscribers() {
        let bus = MemoryEventBus::new(100);
        let event = ChronosEvent::new("LonelyEvent", "Source", json!({}));
        
        let received_count = bus.publish(event).expect("Failed to publish without subs");
        assert_eq!(received_count, 0); // No errors, just 0 deliveries
    }

    #[tokio::test]
    async fn test_bus_graceful_shutdown() {
        let mut sub;
        {
            let bus = MemoryEventBus::new(100);
            sub = bus.subscribe();
            
            let event = ChronosEvent::new("PreShutdown", "Source", json!({}));
            bus.publish(event).unwrap();
            
            // Sub receives the event fine
            let _ = sub.next_event().await.unwrap();
            
            bus.shutdown();
        } // Bus is dropped here

        // Next receive should yield a Shutdown error because the sender was dropped
        let result = sub.next_event().await;
        assert!(matches!(result, Err(BusError::BusShutdown)));
    }
}
