use clap::Parser;
mod cli;
mod file_metadata;
mod protocol;
mod receiver;
mod sender;
mod utils;

use cli::Commands;
use cli::CLI;
use receiver::Receiver;
use sender::Sender;

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = CLI::parse();

    match cli.subcommand {
        Commands::Receive { address } => {
            let receiver = Receiver::new(address);
            receiver.run().await;
        }
        Commands::Send { file } => {
            let sender = Sender::new(file);
            sender.run().await;
        }
    }
}
