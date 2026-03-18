use std::io::{self, BufRead, Write};

use url::Url;

use errors::{Result, anyhow};

/// Wait for user input and return what they typed
fn read_line() -> Result<String> {
    let stdin = io::stdin();
    let stdin = stdin.lock();
    let mut lines = stdin.lines();
    lines
        .next()
        .and_then(|l| l.ok())
        .ok_or_else(|| anyhow!("unable to read from stdin for confirmation"))
}

/// Ask a question to the user where they can write a URL
pub fn ask_url(question: &str, default: &str) -> Result<String> {
    print!("{question} ({default}): ");
    let _ = io::stdout().flush();
    let input = read_line()?;

    match &*input {
        "" => Ok(default.to_string()),
        _ => match Url::parse(&input) {
            Ok(_) => Ok(input),
            Err(_) => {
                println!("Invalid URL: '{input}'");
                ask_url(question, default)
            }
        },
    }
}
