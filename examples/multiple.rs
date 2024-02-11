pub use self::misc::SimpleLogger;
mod misc;

use apocalypse::{Hell, Demon};

// Human demon that echoes a message with its name
struct EchoBot {
    id: usize
}

// Demon implementation for the echobot
impl Demon for EchoBot {
    type Input = String;
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        log::info!("Request for message {} received by replica number {}", message, self.id);
        if self.id != 2 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        message
    }

    #[cfg(feature = "full_log")]
    fn id(&self) -> String {
        format!("EchoBot-{}", self.id)
    }

    #[cfg(feature = "full_log")]
    fn multiple_id() -> &'static str {
        "MultipleEchoBot"
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Debug).init().unwrap();
    // We create one demon
    let mut counter = 0;
    let echo_bot_factory = || {
        let id = counter;
        counter += 1;
        EchoBot {
            id
        }
    };

    // We create a hell for this
    let hell = Hell::new();
    let (gate, jh) = match hell.ignite().await {
        Ok(v) => v,
        Err(e) => panic!("Could not light up hell, {}", e)
    };
    
    // We spawn the demon in the running hell through the gate
    let location = match gate.spawn_multiple(echo_bot_factory, 3).await {
        Ok(v) => v,
        Err(e) => panic!("Could not spawn the demon, {}", e)
    };

    tokio::spawn(async move {
        let (_, _, _, _, _, _) = tokio::join!(
            gate.send(&location, "hello world 1".to_string()),
            gate.send(&location, "hello world 2".to_string()),
            gate.send(&location, "hello world 3".to_string()),
            gate.send(&location, "hello world 4".to_string()),
            gate.send(&location, "hello world 5".to_string()),
            gate.send(&location, "hello world 6".to_string())
        );
    });

    // We wait for all messages to be processed.
    jh.await.unwrap();
}