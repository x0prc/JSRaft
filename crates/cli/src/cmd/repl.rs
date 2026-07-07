use anyhow::Result;
use jsraft_core::ReplSession;
use std::io::{self, Write};

pub async fn execute() -> Result<()> {
    let session = ReplSession::new().await?;
    let mut input = String::new();

    println!("JSRaft REPL");
    println!("Type .exit or .quit to leave.");

    loop {
        print!("> ");
        io::stdout().flush()?;

        input.clear();
        let bytes = io::stdin().read_line(&mut input)?;
        if bytes == 0 {
            break;
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }
        if line == ".exit" || line == ".quit" {
            break;
        }

        match session.eval(line).await {
            Ok(output) if !output.is_empty() => println!("{output}"),
            Ok(_) => {}
            Err(error) => eprintln!("Error: {error}"),
        }
    }

    Ok(())
}
