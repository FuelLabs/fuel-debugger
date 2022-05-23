use cynic::{http::SurfExt, MutationBuilder, QueryBuilder};

use fuel_gql_client::client::schema::{
    primitives::U64, Breakpoint, ContinueTx, ContinueTxArgs, ContractId, EndSession, HexFormatted,
    IdArg, Memory, MemoryArgs, Register, RegisterArgs, RunResult, SetBreakpoint, SetBreakpointArgs,
    SetSingleStepping, SetSingleSteppingArgs, StartSession, StartTx, StartTxArgs,
};

pub mod names;

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

        let operation = StartSession::build(());
        let response = self.http.post(&self.api_url).run_graphql(operation).await?;

        let session_id = response.data.expect("Missing session id").start_session;
        self.session = Some(session_id);
        Ok(())
    }

    pub async fn end_session(&mut self) -> Result<(), surf::Error> {
        let id = self.session.take().expect("No session exists");

        let operation = EndSession::build(IdArg { id });
        let _ = self.http.post(&self.api_url).run_graphql(operation).await?;
        Ok(())
    }

    pub async fn set_breakpoint(
        &mut self,
        bp: fuel_vm::prelude::Breakpoint,
    ) -> Result<(), surf::Error> {
        let id = self.session.clone().expect("No session exists");

        let operation = SetBreakpoint::build(SetBreakpointArgs {
            id,
            bp: Breakpoint {
                contract: ContractId(HexFormatted(*bp.contract())),
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
        let operation = SetSingleStepping::build(SetSingleSteppingArgs { id, enable });
        surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data");
        Ok(())
    }

    pub async fn start_tx(&mut self, tx: &Transaction) -> Result<RunResult, surf::Error> {
        let id = self.session.clone().expect("No session exists");

        let operation = StartTx::build(StartTxArgs {
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

    pub async fn continue_tx(&mut self) -> Result<RunResult, surf::Error> {
        let id = self.session.clone().expect("No session exists");

        let operation = ContinueTx::build(ContinueTxArgs { id });
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
        let operation = Register::build(RegisterArgs {
            id,
            register: U64(reg),
        });
        let response = surf::post(&self.api_url)
            .run_graphql(operation)
            .await?
            .data
            .expect("Missing response data")
            .register;
        Ok(response.0)
    }
    pub async fn read_memory(&mut self, start: u64, size: u64) -> Result<Vec<u8>, surf::Error> {
        let id = self.session.clone().expect("No session exists");

        // Fetch memory range at breakpoint
        let operation = Memory::build(MemoryArgs {
            id,
            start: U64(start),
            size: U64(size),
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
