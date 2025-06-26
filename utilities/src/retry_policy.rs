use std::{error::Error, time::Duration};
use tokio::time::sleep;
use tracing::{error, info};

pub async fn retry_with_backoff<F, Fut, R>(mut f: F, max_retries: u8) -> Result<R, Box<dyn Error>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<R, Box<dyn Error>>>,
{
    let mut curr_try = 1;
    loop {
        match f().await {
            Ok(v) => {
                return Ok(v);
            }
            Err(e) => {
                error!(error=%e,retry=%curr_try,"Error happened while running closure");
                if curr_try == max_retries {
                    error!("Reached max retries return error");
                    return Err(e);
                }
            }
        }
        curr_try += 1;
        let sleep_duration = Duration::from_millis(2u64.pow(curr_try as u32) * 200);
        info!(?sleep_duration, "Waiting before retry");
        sleep(sleep_duration).await;
    }
}
