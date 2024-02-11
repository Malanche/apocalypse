pub use self::misc::SimpleLogger;
mod misc;

use apocalypse::{Hell, Demon};

// Human demon that echoes a message with its name
struct EchoBot;

// Demon implementation for the echobot
impl Demon for EchoBot {
    type Input = (String, std::time::Duration);
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        // handle function waits before replying
        log::info!("actor -> waiting to reply");
        tokio::time::sleep(message.1).await;
        // Will not be reached this time
        log::info!("actor -> replying");
        message.0
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Debug).init().unwrap();
    // We create one demon
    let echo_bot = EchoBot;

    // We create a hell for this
    let hell = Hell::new();
    let jh = {
        let (gate, jh) = match hell.ignite().await {
            Ok(v) => v,
            Err(e) => panic!("Could not light up hell, {}", e)
        };
        
        // We spawn the demon in the running hell through the gate
        let location = match gate.spawn(echo_bot).await {
            Ok(v) => v,
            Err(e) => panic!("Could not spawn the demon, {}", e)
        };
    
        let location_clone = location.clone();
        let gate_clone = gate.clone();
    
        tokio::spawn(async move {
            log::info!("Sending message");
            match gate_clone.send(&location_clone,("hello world".to_string(), std::time::Duration::from_secs(5))).await {
                Ok(reply) => log::info!("reply: {}", reply),
                Err(e) => log::error!("expected recv error, {}", e)
            };
        });
    
        // Force demon vanquish
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _ = gate.vanquish_and_ignore_timeout(&location, std::time::Duration::from_secs(1));
        
        jh
    };

    // We wait for all messages to be processed, should be immediate.
    jh.await.unwrap();
}