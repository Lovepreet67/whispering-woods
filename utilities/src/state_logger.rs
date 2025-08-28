use crate::logger::error;
use crate::result::Result;
use serde::Serialize;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tracing::{info, trace};

pub struct StateLogger<T, U>
where
    T: PartialEq + Serialize + Send + Sync + 'static,
    U: Into<T> + Send + Sync + 'static,
{
    current_state: T,
    file: tokio::fs::File,
    receiver: tokio::sync::mpsc::Receiver<U>,
}
impl<T, U> StateLogger<T, U>
where
    T: PartialEq + Serialize + Send + Sync + 'static,
    U: Into<T> + Send + Sync + 'static,
{
    pub async fn start(state: U, target_file_path: &Path) -> Result<tokio::sync::mpsc::Sender<U>> {
        let file = tokio::fs::File::options()
            .append(true)
            .create(true)
            .open(target_file_path)
            .await?;
        let (tx, rx) = tokio::sync::mpsc::channel::<U>(10);
        let mut state_logger = Self {
            current_state: state.into(),
            file,
            receiver: rx,
        };
        tokio::spawn(async move {
            while let Some(new_state) = state_logger.receiver.recv().await {
                let state = new_state.into();
                trace!("Checking if the state snapshot is same or not");
                if state != state_logger.current_state {
                    match state_logger.update_state(state).await {
                        Ok(_) => {
                            trace!("snapshot written successfully")
                        }
                        Err(e) => {
                            error!("Error while sending the updated state to es, {e}");
                        }
                    }
                }
            }
        });
        Ok(tx)
    }
    async fn update_state(&mut self, new_state: T) -> Result<()> {
        let json_line = serde_json::to_string(&new_state)?;
        self.current_state = new_state;
        self.file.write_all(json_line.as_bytes()).await?;
        self.file.write_all(b"\n").await?;
        Ok(())
    }
}
