use tokio::{
    sync::{
        oneshot::{Sender},
        mpsc::{UnboundedSender}
    }
};
use super::{MiniHellInstruction};

pub(crate) struct DemonChannels {
    /// Channel that receives instructions that execute one after the other
    pub(crate) instructions: UnboundedSender<MiniHellInstruction>,
    /// Killswitch, for demon forced removal
    pub(crate) killswitch: UnboundedSender<Sender<()>>
}