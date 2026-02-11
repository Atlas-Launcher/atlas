use anyhow::{Context, Result};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::time::{Duration, sleep};

const LOG_PATH: &str = "runtime/current/runner.log";
const ERR_PATH: &str = "runtime/current/runner.err.log";
const TAIL_LINES: usize = 200;

pub async fn exec(follow: bool) -> Result<()> {
    if follow {
        follow_logs().await
    } else {
        print_tail().await
    }
}

async fn print_tail() -> Result<()> {
    print_tail_for(LOG_PATH, "stdout").await?;
    print_tail_for(ERR_PATH, "stderr").await?;
    Ok(())
}

async fn follow_logs() -> Result<()> {
    let stdout = tokio::spawn(async { follow_file(LOG_PATH).await });
    let stderr = tokio::spawn(async { follow_file(ERR_PATH).await });

    let _ = tokio::try_join!(stdout, stderr)?;
    Ok(())
}

async fn print_tail_for(path: &str, label: &str) -> Result<()> {
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Log file not found: {}", path))?;
    let lines = content.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(TAIL_LINES);
    for line in &lines[start..] {
        println!("[{label}] {line}");
    }
    Ok(())
}

async fn follow_file(path: &str) -> Result<()> {
    let mut file = File::open(path)
        .await
        .with_context(|| format!("Log file not found: {}", path))?;

    let mut initial = String::new();
    file.read_to_string(&mut initial).await?;
    if !initial.is_empty() {
        print!("{}", initial);
    }

    loop {
        let mut buf = Vec::new();
        let _ = file.seek(SeekFrom::Current(0)).await?;
        file.read_to_end(&mut buf).await?;
        if !buf.is_empty() {
            print!("{}", String::from_utf8_lossy(&buf));
        } else {
            sleep(Duration::from_millis(500)).await;
        }
    }
}
