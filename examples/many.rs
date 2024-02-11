use self::misc::SimpleLogger;
mod misc;

use apocalypse::{Hell, Demon, Gate, Location};

// Human demon that echoes a message with its name
struct ReplaceBot;

// Demon implementation for the replace bot
impl Demon for ReplaceBot {
    type Input = String;
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        message.replace("a", "e").replace("o", "e")
    }
}

// Human demon that echoes a message with its name
struct EmphasisBot;

// Demon implementation for the emphasis bot, which requires the location of the replace bot
impl Demon for EmphasisBot {
    type Input = String;
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        message + "!!!"
    }
}

// Human demon that echoes a message with its name
struct NiceStringBot {
    gate: Gate,
    rb_location: Location<ReplaceBot>,
    eb_location: Location<EmphasisBot>
}

impl NiceStringBot {
    fn new(gate: Gate, rb_location: Location<ReplaceBot>, eb_location: Location<EmphasisBot>) -> NiceStringBot {
        NiceStringBot{gate, rb_location, eb_location}
    }
}

// Demon implementation for the emphasis bot, which requires the location of the replace bot
impl Demon for NiceStringBot {
    type Input = String;
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        let replaced = self.gate.send(&self.rb_location, message).await.unwrap();
        let emphasized = self.gate.send(&self.eb_location, replaced).await.unwrap();
        emphasized
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Debug).init().unwrap();
    // We create a hell for this
    let hell = Hell::new();
    let (gate, jh) = match hell.ignite().await {
        Ok(v) => v,
        Err(e) => panic!("Could not light up hell, {}", e)
    };
    
    // We spawn the demon in the running hell through the gate
    let rb_location = match gate.spawn(ReplaceBot).await {
        Ok(v) => v,
        Err(e) => panic!("Could not spawn the demon, {}", e)
    };

    // We spawn the demon in the running hell through the gate
    let eb_location = match gate.spawn(EmphasisBot).await {
        Ok(v) => v,
        Err(e) => panic!("Could not spawn the demon, {}", e)
    };

    // We spawn the demon in the running hell through the gate
    let nsb_location = match gate.spawn(NiceStringBot::new(gate.clone(), rb_location, eb_location)).await {
        Ok(v) => v,
        Err(e) => panic!("Could not spawn the demon, {}", e)
    };

    tokio::spawn(async move {
        let m1 = gate.send(&nsb_location, "hello world".to_string()).await.unwrap();
        // And check that it is correct
        log::info!("Received chain {}", m1);
        assert_eq!("helle werld!!!", &m1);
        // And we kill the emphasis bot to vanquish all gates
        gate.vanquish_and_ignore(&nsb_location).unwrap();
    });

    // We wait for all messages to be processed.
    jh.await.unwrap();
}