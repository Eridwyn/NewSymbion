// Clock abstraction pour testabilite
// Spec: PR3 P0 v3.1 REFINED

use std::sync::Arc;
use time::OffsetDateTime;
use parking_lot::RwLock;

/// Trait d'abstraction horloge
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> OffsetDateTime;
}

/// Horloge systeme (production)
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Horloge mockable (tests avec time travel)
pub struct MockClock {
    current_time: Arc<RwLock<OffsetDateTime>>,
}

impl MockClock {
    pub fn new(initial: OffsetDateTime) -> Self {
        Self {
            current_time: Arc::new(RwLock::new(initial)),
        }
    }

    /// Avancer le temps (time travel)
    pub fn advance(&self, duration: time::Duration) {
        let mut time = self.current_time.write();
        *time = *time + duration;
    }

    /// Definir temps exact
    pub fn set(&self, time: OffsetDateTime) {
        *self.current_time.write() = time;
    }

    /// Obtenir temps actuel
    pub fn get(&self) -> OffsetDateTime {
        *self.current_time.read()
    }
}

impl Clock for MockClock {
    fn now_utc(&self) -> OffsetDateTime {
        *self.current_time.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn test_system_clock() {
        let clock = SystemClock;
        let now = clock.now_utc();

        // Verifie que c'est proche du temps reel (tolerance 1s)
        let real_now = OffsetDateTime::now_utc();
        let diff = (now - real_now).whole_seconds().abs();
        assert!(diff < 1);
    }

    #[test]
    fn test_mock_clock_advance() {
        let initial = datetime!(2025-11-01 10:00 UTC);
        let clock = MockClock::new(initial);

        assert_eq!(clock.get(), initial);

        // Avancer 1 heure
        clock.advance(time::Duration::hours(1));
        assert_eq!(clock.get(), datetime!(2025-11-01 11:00 UTC));

        // Avancer 30 minutes
        clock.advance(time::Duration::minutes(30));
        assert_eq!(clock.get(), datetime!(2025-11-01 11:30 UTC));
    }

    #[test]
    fn test_mock_clock_set() {
        let initial = datetime!(2025-11-01 10:00 UTC);
        let clock = MockClock::new(initial);

        // Set temps exact
        let new_time = datetime!(2025-12-25 18:30 UTC);
        clock.set(new_time);
        assert_eq!(clock.get(), new_time);
    }

    #[test]
    fn test_mock_clock_trait() {
        let initial = datetime!(2025-11-01 10:00 UTC);
        let clock = MockClock::new(initial);

        // Via trait Clock
        let now = clock.now_utc();
        assert_eq!(now, initial);
    }
}
