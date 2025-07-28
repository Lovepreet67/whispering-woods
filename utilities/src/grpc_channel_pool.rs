use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};

use crate::result::Result;
use crate::retry_policy::retry_with_backoff;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};
use tracing::{Instrument, Span, instrument, trace};

#[derive(Clone, Debug)]
pub struct GrpcChannelPool {
    store: Arc<Mutex<HashMap<String, Channel>>>,
}
impl GrpcChannelPool {
    fn new() -> Self {
        Self {
            store: Arc::default(),
        }
    }
    #[instrument(name = "grpc_pool_get_channel", skip(self))]
    pub async fn get_channel(&self, addrs: &str) -> Result<Channel> {
        if let Some(chnl) = self.store.lock().await.get(addrs) {
            return Ok(chnl.clone());
        }
        trace!("Creating endpoint for channel since channel is not present already");
        let endpoint = Endpoint::from_str(addrs)
            .map_err(|e| format!("Error while creating an endpoint {e} for location {addrs}"))?
            .connect_timeout(Duration::from_secs(5));

        let chnl = retry_with_backoff(
            || {
                async {
                    endpoint
                        .connect()
                        .await
                        .map_err(|e| format!("Error while connecting to address {e:?}").into())
                }
                .instrument(Span::current())
            },
            3,
        )
        .await
        .unwrap();
        self.store
            .lock()
            .await
            .insert(addrs.to_owned(), chnl.clone());
        Ok(chnl)
    }
}

pub static GRPC_CHANNEL_POOL: once_cell::sync::Lazy<GrpcChannelPool> =
    once_cell::sync::Lazy::new(GrpcChannelPool::new);
