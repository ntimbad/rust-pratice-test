use crate::runner::Runner;
use std::env;
use std::error::Error;
use std::path::PathBuf;

mod client_account;
mod constants;
mod error;
mod runner;
mod transaction;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let command_line_args: Vec<String> = env::args().collect();
    let input_csv_file_path = PathBuf::from(&command_line_args[1]);
    let mut runner = Runner::new(input_csv_file_path);
    runner.run().await?;
    runner.print_all_accounts().await;
    Ok(())
}
