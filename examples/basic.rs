//! Basic framework use, for a single actor

use apocalypse::{Hell, Demon};

// Human demon that echoes a message with its name
struct Human {
    name: String
}

// Demon implementation for the human
#[async_trait::async_trait]
impl Demon for Human {
    type Input = String;
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        format!("Hey, {} here: {}", self.name, &message)
    }
}

#[tokio::main]
async fn main() {
    // We create one demon
    let carlos = Human {
        name: "Carlos".to_string()
    };

    // We create a hell for this
    let hell = Hell::new();
    let (gate, jh) = match hell.fire().await {
        Ok(v) => v,
        Err(e) => panic!("Could not light up hell, {}", e)
    };
    
    // We spawn the demon in the running hell through the gate
    let location = match gate.spawn(carlos).await {
        Ok(v) => v,
        Err(e) => panic!("Could not spawn the demon, {}", e)
    };

    tokio::spawn(async move {
        let m1 = gate.send(&location, "hello world".to_string()).await.unwrap();
        // We print the message just for show
        println!("{}", &m1);
        // And check that it is correct
        assert_eq!("Hey, Carlos here: hello world", &m1);
    });

    // We wait for all messages to be processed.
    jh.await.unwrap();
}