use parking_lot::Mutex;
use std::sync::Arc;

pub type Shared<T> = Arc<Mutex<T>>;

pub fn new_state<T>(value: T) -> Shared<T> {
    Arc::new(Mutex::new(value))
}
