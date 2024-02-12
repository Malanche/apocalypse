use chrono::{DateTime, Utc};
#[cfg(feature = "serialize")]
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct HellStats {
    /// Amount of spawned demons through the lifetime of this hell instance
    pub spawned_demons: usize,
    /// Amount of active demons at the time of the call
    pub active_demons: usize,
    /// Zombie demons. That is, the instance still exists for whatever reason, but are no longer rechable
    pub zombie_demons: usize,
    /// Total number of messages delivered to demons
    pub successful_messages: usize,
    /// Total number of messages whose deivery failed
    pub failed_messages: usize,
    /// Time of ignition of the hell instance
    pub ignition_time: DateTime<Utc>
}