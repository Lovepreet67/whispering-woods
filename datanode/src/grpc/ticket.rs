use std::sync::Arc;
use tonic::{Status, service::Interceptor};
use utilities::{logger::info, ticket::ticket_decrypter::TicketDecrypter};

#[derive(Clone)]
pub struct TicketIntercepter {
    ticket_decrypter: Arc<Box<dyn TicketDecrypter>>,
}
impl TicketIntercepter {
    pub fn new(ticket_decrypter: Arc<Box<dyn TicketDecrypter>>) -> Self {
        Self { ticket_decrypter }
    }
}

impl Interceptor for TicketIntercepter {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(encoded_ticket) = request.metadata().get("ticket") {
            let server_ticket = self
                .ticket_decrypter
                .decrypt_server_ticket(encoded_ticket.to_str().unwrap_or(""))
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::PermissionDenied,
                        format!("Error while decrypting/decoding server ticket {e}"),
                    )
                })?;
            info!("Got server ticket {:?}", server_ticket);
            request.extensions_mut().insert(server_ticket);
            Ok(request)
        } else {
            Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "Please add Ticket",
            ))
        }
    }
}
