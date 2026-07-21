// Minimal test: confirms the server accepts queries and returns FlightData.
use arrow_flight::{FlightDescriptor, flight_service_client::FlightServiceClient};
use arrow_flight::sql::{CommandStatementQuery, ProstMessageExt};
use futures::StreamExt;
use prost::Message;
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Endpoint::new("http://127.0.0.1:50051")?.connect().await?;
    let mut client = FlightServiceClient::new(channel);

    for sql in [
        "SELECT count(*) FROM ducklake.main.loans",
        "SELECT property_state, count(*) as cnt FROM ducklake.main.loans GROUP BY property_state ORDER BY cnt DESC LIMIT 5",
    ] {
        println!("SQL: {sql}");
        let query = CommandStatementQuery { query: sql.to_string(), transaction_id: None };
        let fd = FlightDescriptor::new_cmd(prost::bytes::Bytes::from(query.as_any().encode_to_vec()));
        let info = client.get_flight_info(fd).await?.into_inner();
        println!("  endpoints: {}", info.endpoint.len());

        for ep in &info.endpoint {
            if let Some(ticket) = &ep.ticket {
                let stream = client.do_get(ticket.clone()).await?.into_inner();
                let msgs: Vec<_> = stream.collect().await;
                println!("  messages: {}", msgs.len());
                for (i, msg) in msgs.iter().enumerate() {
                    if let Ok(fd) = msg {
                        println!("    [{i}] header={}B body={}B", fd.data_header.len(), fd.data_body.len());
                    }
                }
            }
        }
    }

    println!("\nServer works!");
    Ok(())
}
