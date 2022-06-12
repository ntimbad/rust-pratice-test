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

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::{BigDecimal, FromPrimitive};
    use std::process::{Command, Stdio};
    use std::fs::{File};
    use std::os::unix::prelude::{FromRawFd, IntoRawFd};
    use std::fs;

    #[tokio::test]
    async fn sample_test() -> Result<(), Box<dyn Error>> {
        let input_csv_file_path = PathBuf::from("data/sample.csv");
        let mut runner = Runner::new(input_csv_file_path);
        runner.run().await?;
        let x = runner.get_cloned_account_snapshot(1).await;
        let y = runner.get_cloned_account_snapshot(2).await;
        assert_eq!(x.as_ref().unwrap().available, BigDecimal::from_f64(100.5).unwrap());
        assert_eq!(x.as_ref().unwrap().held, BigDecimal::from(0));
        assert_eq!(y.as_ref().unwrap().available, BigDecimal::from(1));
        assert_eq!(y.as_ref().unwrap().held, BigDecimal::from(0));
        Ok(())
    }

    #[tokio::test]
    async fn threading_test() -> Result<(), Box<dyn Error>> {
        let file = File::create("output.csv").expect("couldn't create file");

        let _ = Command::new("./generate_data.sh")
            .stdout(unsafe { Stdio::from_raw_fd(file.into_raw_fd()) })
            .output()
            .expect("failed");

        for _ in 0..5 {
            let input_csv_file_path = PathBuf::from("output.csv");
            let mut runner = Runner::new(input_csv_file_path);
            runner.run().await?;
            let x = runner.get_cloned_account_snapshot(1).await;
            assert_eq!(x.as_ref().unwrap().available, BigDecimal::from(10499)); // Since 1st line is ignored
        }

        fs::remove_file("output.csv")?;
        Ok(())
    }

}
