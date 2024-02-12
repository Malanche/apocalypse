pub use self::misc::SimpleLogger;
mod misc;

use apocalypse::{Hell, Demon};

// Human demon that echoes a message with its name
struct EchoBot;

// Demon implementation for the echobot
impl Demon for EchoBot {
    type Input = String;
    type Output = String;

    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        log::info!("Received request");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        log::info!("Replying");
        message
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

        let gate_clone = gate.clone();
    
        tokio::spawn(async move {
            let m1 = gate_clone.send(&location, "hello world".to_string()).await.unwrap();
            // And check that it is correct
            assert_eq!("hello world", &m1);
        });

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let stats = gate.stats().await.unwrap();

        log::info!("stats: {:?}", stats);

        jh
    };

    // We wait for all messages to be processed.
    jh.await.unwrap();
}