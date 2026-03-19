use std::collections::VecDeque;

/// Trait that all game events must implement
pub trait GameEvent: std::fmt::Debug + Clone + Send + Sync {}

/// Generic event queue that can hold any type of game event
#[derive(Debug, Default)]
pub struct EventQueue<T: GameEvent> {
    events: VecDeque<T>,
}

impl<T: GameEvent> EventQueue<T> {
    /// Create a new empty event queue
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    /// Push an event to the back of the queue
    pub fn push(&mut self, event: T) {
        self.events.push_back(event);
    }

    /// Pop an event from the front of the queue
    pub fn pop(&mut self) -> Option<T> {
        self.events.pop_front()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get the number of events in the queue
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Clear all events from the queue
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Drain all events and return them as a vector
    pub fn drain(&mut self) -> Vec<T> {
        self.events.drain(..).collect()
    }
}

/// Trait for handling specific types of game events
pub trait EventHandler<T: GameEvent> {
    /// Handle a single event
    fn handle(&mut self, event: &T);

    /// Handle multiple events at once (default implementation processes them one by one)
    fn handle_batch(&mut self, events: &[T]) {
        for event in events {
            self.handle(event);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSwitchRequest {
    pub scene_name: String,
    pub spawn_point_id: String,
}

/// Game update result that includes both state changes and events
#[derive(Debug, Default)]
pub struct GameUpdateResult<T: GameEvent> {
    /// Whether the player moved this frame
    pub player_moved: bool,
    /// Events that were generated this frame
    pub events: Vec<T>,
    /// Optional deferred scene-switch request to be applied by the runtime layer.
    pub scene_switch_request: Option<SceneSwitchRequest>,
}

impl<T: GameEvent> GameUpdateResult<T> {
    /// Create a new result with no movement and no events
    pub fn new() -> Self {
        Self {
            player_moved: false,
            events: Vec::new(),
            scene_switch_request: None,
        }
    }

    /// Create a result with movement status
    pub fn with_movement(player_moved: bool) -> Self {
        Self {
            player_moved,
            events: Vec::new(),
            scene_switch_request: None,
        }
    }

    /// Add an event to this result
    pub fn add_event(&mut self, event: T) {
        self.events.push(event);
    }

    /// Add multiple events to this result
    pub fn add_events(&mut self, events: impl IntoIterator<Item = T>) {
        self.events.extend(events);
    }

    pub fn request_scene_switch(
        &mut self,
        scene_name: impl Into<String>,
        spawn_point_id: impl Into<String>,
    ) {
        self.scene_switch_request = Some(SceneSwitchRequest {
            scene_name: scene_name.into(),
            spawn_point_id: spawn_point_id.into(),
        });
    }
}
