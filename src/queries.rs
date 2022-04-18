//! GraphQL queries against fuel-core debug interface

pub use self::inner::*;

/// This is an inline module because proc-macros on stable require it.
/// https://github.com/rust-lang/rust/issues/54727
#[cynic::schema_for_derives(
    file = "../fuel-core/fuel-core/schema.graphql",
    module = "crate::schema"
)]
mod inner {

    #[derive(cynic::Scalar, Debug, Clone)]
    pub struct U64(String);
    impl U64 {
        pub fn new(v: u64) -> Self {
            Self(v.to_string())
        }

        pub fn value(&self) -> u64 {
            self.0.parse().unwrap()
        }
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct RegisterArguments {
        pub id: cynic::Id,
        pub reg: U64,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Query", argument_struct = "RegisterArguments")]
    pub struct Register {
        #[arguments(id = &args.id, register = &args.reg)]
        pub register: U64,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct MemoryArguments {
        pub id: cynic::Id,
        pub start: U64,
        pub size: U64,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Query", argument_struct = "MemoryArguments")]
    pub struct Memory {
        #[arguments(id = &args.id, start = &args.start, size = &args.size)]
        pub memory: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Mutation")]
    pub struct StartSession {
        pub start_session: cynic::Id,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct EndSessionArguments {
        pub id: cynic::Id,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Mutation", argument_struct = "EndSessionArguments")]
    pub struct EndSession {
        #[arguments(id = &args.id)]
        pub end_session: bool,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct SetBreakpointArguments {
        pub id: cynic::Id,
        pub bp: Breakpoint,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Mutation", argument_struct = "SetBreakpointArguments")]
    pub struct SetBreakpoint {
        #[arguments(id = &args.id, breakpoint = &args.bp)]
        pub set_breakpoint: bool,
    }

    #[derive(cynic::InputObject, Debug)]
    pub struct Breakpoint {
        pub contract: Vec<i32>,
        pub pc: i32,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct SetSingleSteppingArguments {
        pub id: cynic::Id,
        pub enable: bool,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        graphql_type = "Mutation",
        argument_struct = "SetSingleSteppingArguments"
    )]
    pub struct SetSingleStepping {
        #[arguments(id = &args.id, enable = &args.enable)]
        pub set_single_stepping: bool,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct StartTxArguments {
        pub id: cynic::Id,
        pub tx: String,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Mutation", argument_struct = "StartTxArguments")]
    pub struct StartTx {
        #[arguments(id = &args.id, tx_json = &args.tx)]
        pub start_tx: RunResult,
    }

    #[derive(cynic::FragmentArguments, Debug)]
    pub struct ContinueTxArguments {
        pub id: cynic::Id,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(graphql_type = "Mutation", argument_struct = "ContinueTxArguments")]
    pub struct ContinueTx {
        #[arguments(id = &args.id)]
        pub continue_tx: RunResult,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct RunResult {
        pub breakpoint: Option<OutputBreakpoint>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct OutputBreakpoint {
        pub contract: OutputContractId,
        pub pc: i32,
    }

    #[derive(cynic::QueryFragment, Debug)]
    pub struct OutputContractId {
        pub value: Vec<i32>,
    }
}
