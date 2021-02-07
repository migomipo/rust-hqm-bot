use std::env;
use std::net::{IpAddr, SocketAddr};
use crate::hqm_game::{HQMMessage, HQMPlayerInput, HQMGameState};
use crate::hqm_bot::{HQMBotLogic, HQMBotSession};

mod hqm_parse;
mod hqm_bot;
mod hqm_game;

struct EmptyBot {
}

impl HQMBotLogic for EmptyBot {
    fn new_game(&mut self) {

    }

    fn tick(&mut self, gamestate: &HQMGameState, messages: &[HQMMessage]) -> (HQMPlayerInput, Option<String>) {
        let input = Default::default();
        let chat = if gamestate.step % 1000 == 700 {
            Some("Test".to_owned())
        } else {
            None
        };
        (input, chat)
    }

}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let addr = args[1].parse::<IpAddr> ().unwrap();
    let port = args[2].parse::<u16> ().unwrap();
    let name = args[3].clone();
    let addr = SocketAddr::new(addr, port);

    HQMBotSession::new(name, EmptyBot {}).start(addr).await?;

    Ok(())


}
