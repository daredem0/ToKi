use toki_core::game::AudioEvent;
use toki_core::{EventHandler, EventQueue, GameEvent, GameUpdateResult};

// Test event implementation
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum TestEvent {
    #[default]
    SimpleEvent,
    EventWithData(String),
    NumberEvent(i32),
}

impl GameEvent for TestEvent {}

// Test event handler implementation
struct TestEventHandler {
    received_events: Vec<TestEvent>,
    handle_count: usize,
}

impl TestEventHandler {
    fn new() -> Self {
        Self {
            received_events: Vec::new(),
            handle_count: 0,
        }
    }

    fn events(&self) -> &Vec<TestEvent> {
        &self.received_events
    }

    fn handle_count(&self) -> usize {
        self.handle_count
    }
}

impl EventHandler<TestEvent> for TestEventHandler {
    fn handle(&mut self, event: &TestEvent) {
        self.received_events.push(event.clone());
        self.handle_count += 1;
    }
}

#[test]
fn event_queue_new_is_empty() {
    let queue: EventQueue<TestEvent> = EventQueue::new();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn event_queue_default_is_empty() {
    let queue: EventQueue<TestEvent> = EventQueue::default();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn event_queue_push_and_pop_single_event() {
    let mut queue = EventQueue::new();
    let event = TestEvent::SimpleEvent;

    queue.push(event.clone());
    assert!(!queue.is_empty());
    assert_eq!(queue.len(), 1);

    let popped = queue.pop();
    assert_eq!(popped, Some(event));
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn event_queue_fifo_ordering() {
    let mut queue = EventQueue::new();

    // Push events in order
    queue.push(TestEvent::EventWithData("first".to_string()));
    queue.push(TestEvent::EventWithData("second".to_string()));
    queue.push(TestEvent::EventWithData("third".to_string()));

    assert_eq!(queue.len(), 3);

    // Pop events should be in FIFO order
    assert_eq!(
        queue.pop(),
        Some(TestEvent::EventWithData("first".to_string()))
    );
    assert_eq!(
        queue.pop(),
        Some(TestEvent::EventWithData("second".to_string()))
    );
    assert_eq!(
        queue.pop(),
        Some(TestEvent::EventWithData("third".to_string()))
    );
    assert_eq!(queue.pop(), None);
}

#[test]
fn event_queue_pop_empty_returns_none() {
    let mut queue: EventQueue<TestEvent> = EventQueue::new();
    assert_eq!(queue.pop(), None);
}

#[test]
fn event_queue_clear_empties_queue() {
    let mut queue = EventQueue::new();
    queue.push(TestEvent::SimpleEvent);
    queue.push(TestEvent::NumberEvent(42));

    assert_eq!(queue.len(), 2);

    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
    assert_eq!(queue.pop(), None);
}

#[test]
fn event_queue_drain_returns_all_events() {
    let mut queue = EventQueue::new();
    let events = vec![
        TestEvent::SimpleEvent,
        TestEvent::EventWithData("test".to_string()),
        TestEvent::NumberEvent(123),
    ];

    for event in &events {
        queue.push(event.clone());
    }

    let drained = queue.drain();
    assert_eq!(drained, events);
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
}

#[test]
fn event_queue_drain_empty_returns_empty_vec() {
    let mut queue: EventQueue<TestEvent> = EventQueue::new();
    let drained = queue.drain();
    assert!(drained.is_empty());
}

#[test]
fn event_handler_receives_single_event() {
    let mut handler = TestEventHandler::new();
    let event = TestEvent::SimpleEvent;

    handler.handle(&event);

    assert_eq!(handler.handle_count(), 1);
    assert_eq!(handler.events().len(), 1);
    assert_eq!(handler.events()[0], event);
}

#[test]
fn event_handler_receives_multiple_events() {
    let mut handler = TestEventHandler::new();
    let events = vec![
        TestEvent::SimpleEvent,
        TestEvent::EventWithData("test".to_string()),
        TestEvent::NumberEvent(42),
    ];

    for event in &events {
        handler.handle(event);
    }

    assert_eq!(handler.handle_count(), 3);
    assert_eq!(handler.events(), &events);
}

#[test]
fn event_handler_batch_processing() {
    let mut handler = TestEventHandler::new();
    let events = vec![
        TestEvent::EventWithData("batch1".to_string()),
        TestEvent::EventWithData("batch2".to_string()),
        TestEvent::EventWithData("batch3".to_string()),
    ];

    handler.handle_batch(&events);

    assert_eq!(handler.handle_count(), 3);
    assert_eq!(handler.events(), &events);
}

#[test]
fn game_update_result_new_has_defaults() {
    let result: GameUpdateResult<TestEvent> = GameUpdateResult::new();

    assert!(!result.player_moved);
    assert!(result.events.is_empty());
}

#[test]
fn game_update_result_with_movement() {
    let result: GameUpdateResult<TestEvent> = GameUpdateResult::with_movement(true);

    assert!(result.player_moved);
    assert!(result.events.is_empty());
}

#[test]
fn game_update_result_add_single_event() {
    let mut result = GameUpdateResult::new();
    let event = TestEvent::SimpleEvent;

    result.add_event(event.clone());

    assert_eq!(result.events.len(), 1);
    assert_eq!(result.events[0], event);
}

#[test]
fn game_update_result_add_multiple_events() {
    let mut result = GameUpdateResult::new();
    let events = vec![TestEvent::SimpleEvent, TestEvent::NumberEvent(100)];

    for event in &events {
        result.add_event(event.clone());
    }

    assert_eq!(result.events.len(), 2);
    assert_eq!(result.events, events);
}

#[test]
fn game_update_result_add_events_batch() {
    let mut result = GameUpdateResult::new();
    let events = vec![
        TestEvent::EventWithData("first".to_string()),
        TestEvent::EventWithData("second".to_string()),
    ];

    result.add_events(events.clone());

    assert_eq!(result.events, events);
}

#[test]
fn game_update_result_add_events_iterator() {
    let mut result = GameUpdateResult::new();
    let events = vec![
        TestEvent::NumberEvent(1),
        TestEvent::NumberEvent(2),
        TestEvent::NumberEvent(3),
    ];

    result.add_events(events.iter().cloned());

    assert_eq!(result.events, events);
}

#[test]
fn game_update_result_combined_movement_and_events() {
    let mut result = GameUpdateResult::with_movement(true);

    result.add_event(TestEvent::SimpleEvent);
    result.add_event(TestEvent::EventWithData("moved".to_string()));

    assert!(result.player_moved);
    assert_eq!(result.events.len(), 2);
    assert_eq!(result.events[0], TestEvent::SimpleEvent);
    assert_eq!(
        result.events[1],
        TestEvent::EventWithData("moved".to_string())
    );
}

// Integration tests with actual AudioEvent
#[test]
fn audio_event_implements_game_event() {
    let event = AudioEvent::PlayerWalk;
    // Test that AudioEvent implements GameEvent by using it in generic functions
    let mut queue = EventQueue::new();
    queue.push(event);
    assert_eq!(queue.len(), 1);
}

#[test]
fn audio_event_in_game_update_result() {
    let mut result = GameUpdateResult::new();

    result.add_event(AudioEvent::PlayerWalk);
    result.add_event(AudioEvent::PlayerCollision);
    result.add_event(AudioEvent::BackgroundMusic("test_music".to_string()));

    assert_eq!(result.events.len(), 3);
    assert!(matches!(result.events[0], AudioEvent::PlayerWalk));
    assert!(matches!(result.events[1], AudioEvent::PlayerCollision));
    assert!(matches!(result.events[2], AudioEvent::BackgroundMusic(_)));
}

#[test]
fn audio_event_queue_operations() {
    let mut queue = EventQueue::new();

    queue.push(AudioEvent::PlayerWalk);
    queue.push(AudioEvent::PlayerCollision);

    assert_eq!(queue.len(), 2);

    let first = queue.pop().unwrap();
    assert!(matches!(first, AudioEvent::PlayerWalk));

    let second = queue.pop().unwrap();
    assert!(matches!(second, AudioEvent::PlayerCollision));

    assert!(queue.is_empty());
}

#[test]
fn event_queue_handles_large_batch() {
    let mut queue = EventQueue::new();
    let mut expected_events = Vec::new();

    // Add 1000 events
    for i in 0..1000 {
        let event = TestEvent::NumberEvent(i);
        queue.push(event.clone());
        expected_events.push(event);
    }

    assert_eq!(queue.len(), 1000);

    let drained = queue.drain();
    assert_eq!(drained, expected_events);
    assert!(queue.is_empty());
}

#[test]
fn event_system_debug_formatting() {
    let queue: EventQueue<TestEvent> = EventQueue::new();
    let result: GameUpdateResult<TestEvent> = GameUpdateResult::new();

    // Should not panic - testing Debug implementation
    let _queue_debug = format!("{:?}", queue);
    let _result_debug = format!("{:?}", result);
}

#[test]
fn event_system_clone_behavior() {
    let event = TestEvent::EventWithData("original".to_string());
    let cloned = event.clone();

    assert_eq!(event, cloned);

    let mut result = GameUpdateResult::new();
    result.add_event(event);
    result.add_event(cloned);

    assert_eq!(result.events.len(), 2);
}
