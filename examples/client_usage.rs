use fuel_debugger::{Client, Transaction};
use fuel_vm::state::Breakpoint;

#[tokio::main]
async fn main() {
    run_example().await.expect("Running example failed");
}

async fn run_example() -> Result<(), surf::Error> {
    let mut client = Client::new(surf::Url::parse("http://localhost:4000/graphql").unwrap());

    client.start_session().await?;

    client.set_breakpoint(Breakpoint::script(0)).await?;

    let tx: Transaction = serde_json::from_str(include_str!("example_tx.json")).unwrap();
    let status = client.start_tx(&tx).await?;
    assert!(status.breakpoint.is_some());

    let value = client.read_register(12).await?;
    println!("reg[12] = {}", value);

    let mem = client.read_memory(0x10, 0x20).await?;
    println!("mem[0x10..0x30] = {:?}", mem);

    client.set_single_stepping(true).await?;

    let status = client.continue_tx().await?;
    assert!(status.breakpoint.is_some());

    client.set_single_stepping(false).await?;

    let status = client.continue_tx().await?;
    assert!(status.breakpoint.is_none());

    client.end_session().await?;

    Ok(())
}
