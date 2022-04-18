pub mod names;
mod schema {
    cynic::use_schema!("schema.graphql");
}
mod queries;

use cynic::{http::SurfExt, MutationBuilder, QueryBuilder};

// Re-exports
pub use fuel_vm::prelude::Transaction;

pub struct Client {
    /// A HTTP client
    http: surf::Client,
    /// GraphQL API endpoint
    api_url: surf::Url,
    /// Active debugger session, if any
    session: Option<cynic::Id>,
}

impl Client {
    pub fn new(api_url: surf::Url) -> Self {
        Self::new_config(api_url, surf::Config::new())
    }

    pub fn new_config(api_url: surf::Url, config: surf::Config) -> Self {
        let client: surf::Client = config
            .try_into()
            .expect("Unable to instantiate a HTTP client");

        Self {
            http: client,
            api_url,
            session: None,
        }
    }

    pub async fn start_session(&mut self) -> Result<(), surf::Error> {
        assert!(self.session.is_none(), "A session already exists");

        let operation = queries::StartSession::build(());
        let response = self.http.post(&self.api_url).run_graphql(operation).await?;

        let session_id = response.data.expect("Missing session id").start_session;
        self.session = Some(session_id);
        Ok(())
    }

    pub async fn end_session(&mut self) -> Result<(), surf::Error> {
        let id = self.session.take().expect("No session exists");

        let operation = queries::EndSession::build(queries::EndSessionArguments { id });
        let _ = self.http.post(&self.api_url).run_graphql(operation).await?;
        Ok(())
    }

    pub async fn set_breakpoint(
        &mut self,
        bp: fuel_vm::prelude::Breakpoint,
    ) -> Result<(), surf::Error> {
        let id = self.session.clone().expect("No session exists");

        let operation = queries::SetBreakpoint::build(queries::SetBreakpointArguments {
            id,
            bp: queries::Breakpoint {
                contract: bp.contract().iter().map(|x| (*x) as i32).collect(),
                pc: bp
                    .pc()
                    .try_into()
                    .expect("pc outside i32 range is not supported yet"),
            },
        });
        let response = surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data");

        assert!(
            response.set_breakpoint,
            "Setting breakpoint returned invalid reply"
        );
        Ok(())
    }

    pub async fn set_single_stepping(&mut self, enable: bool) -> Result<(), surf::Error> {
        let id = self.session.clone().expect("No session exists");

        // Disable single-stepping
        let operation =
            queries::SetSingleStepping::build(queries::SetSingleSteppingArguments { id, enable });
        surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data");
        Ok(())
    }

    pub async fn start_tx(&mut self, tx: &Transaction) -> Result<queries::RunResult, surf::Error> {
        let id = self.session.clone().expect("No session exists");

        let operation = queries::StartTx::build(queries::StartTxArguments {
            id,
            tx: serde_json::to_string(tx).expect("Couldn't serialize tx to json"),
        });
        let response = surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data")
            .start_tx;
        Ok(response)
    }

    pub async fn continue_tx(&mut self) -> Result<queries::RunResult, surf::Error> {
        let id = self.session.clone().expect("No session exists");

        let operation = queries::ContinueTx::build(queries::ContinueTxArguments { id });
        let response = surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data")
            .continue_tx;
        Ok(response)
    }

    pub async fn read_register(&mut self, reg: u64) -> Result<u64, surf::Error> {
        let id = self.session.clone().expect("No session exists");

        // Fetch register at breakpoint
        let operation = queries::Register::build(queries::RegisterArguments {
            id,
            reg: queries::U64::new(reg),
        });
        let response = surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data")
            .register;
        Ok(response.value())
    }
    pub async fn read_memory(&mut self, start: u64, size: u64) -> Result<Vec<u8>, surf::Error> {
        let id = self.session.clone().expect("No session exists");

        // Fetch memory range at breakpoint
        let operation = queries::Memory::build(queries::MemoryArguments {
            id,
            start: queries::U64::new(start),
            size: queries::U64::new(size),
        });
        let response = surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data")
            .memory;
        Ok(serde_json::from_str(&response).expect("Invalid JSON array"))
    }
}
