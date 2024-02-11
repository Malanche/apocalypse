use chrono::{DateTime, Utc};

pub struct HellStats {
    /// Amount of active demons at the time of the call
    pub active_demons: usize,
    /// Zombie demons. That is, the instance still exists for whatever reason, but are no longer rechable
    pub zombie_demons: usize,
    /// Time of ignition of the hell instance
    pub ignition_time: DateTime<Utc>
}