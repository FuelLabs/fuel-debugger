use clap::Parser;
use fuel_debugger::names::register_name;
use shellfish::async_fn;
use shellfish::{Command as ShCommand, Shell};
use std::error::Error;
use std::fmt;

use fuel_vm::consts::{VM_MAX_RAM, VM_REGISTER_COUNT, WORD_SIZE};
use fuel_vm::prelude::ContractId;

use fuel_debugger::{names, Client, Transaction};

#[derive(Parser, Debug)]
pub struct Opt {
    #[clap(default_value = "http://127.0.0.1:4000/graphql")]
    pub api_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Opt::parse();

    let mut shell = Shell::new_async(
        State {
            client: Client::new(surf::Url::parse(&config.api_url)?),
        },
        ">> ",
    );

    macro_rules! command {
        ($f:ident, $help:literal, $names:expr) => {
            for c in $names {
                shell.commands.insert(
                    c,
                    ShCommand::new_async($help.to_string(), async_fn!(State, $f)),
                );
            }
        };
    }

    command!(
        cmd_start_tx,
        "path/to/tx.json -- run until next breakpoint or termination",
        ["n", "tx", "new_tx", "start_tx"]
    );
    command!(
        cmd_continue,
        "-- run until next breakpoint or termination",
        ["c", "continue"]
    );
    command!(
        cmd_step,
        "[on|off] -- turn single-stepping on or off",
        ["s", "step"]
    );
    command!(
        cmd_breakpoint,
        "[contract_id] offset -- set a breakpoint",
        ["b", "breakpoint"]
    );
    command!(
        cmd_registers,
        "[regname ...] -- dump registers",
        ["r", "reg", "register", "registers"]
    );
    command!(cmd_memory, "[offset] limit -- dump memory", ["m", "memory"]);

    shell.state.client.start_session().await?;
    shell.run_async().await?;
    shell.state.client.end_session().await?;
    Ok(())
}

struct State {
    client: Client,
}

#[derive(Debug)]
enum ArgError {
    Invalid,
    NotEnough,
    TooMany,
}
impl Error for ArgError {}
impl fmt::Display for ArgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "Invalid argument"),
            Self::NotEnough => write!(f, "Not enough arguments"),
            Self::TooMany => write!(f, "Too many arguments"),
        }
    }
}

async fn cmd_start_tx(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    let path_to_tx_json = args.pop().ok_or_else(|| Box::new(ArgError::NotEnough))?;
    if !args.is_empty() {
        return Err(Box::new(ArgError::TooMany));
    }

    let tx_json = std::fs::read(path_to_tx_json)?;
    let tx: Transaction = serde_json::from_slice(&tx_json).unwrap();
    let status = state.client.start_tx(&tx).await?;
    println!("{:?}", status); // TODO: pretty-print

    Ok(())
}

async fn cmd_continue(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    if !args.is_empty() {
        return Err(Box::new(ArgError::TooMany));
    }

    let status = state.client.continue_tx().await?;
    println!("{:?}", status); // TODO: pretty-print

    Ok(())
}

async fn cmd_step(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    if args.len() > 1 {
        return Err(Box::new(ArgError::TooMany));
    }

    state
        .client
        .set_single_stepping(
            args.get(0)
                .map(|v| !["off", "no", "disable"].contains(&v.as_str()))
                .unwrap_or(true),
        )
        .await?;
    Ok(())
}

async fn cmd_breakpoint(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    let offset = args.pop().ok_or_else(|| Box::new(ArgError::NotEnough))?;
    let contract_id = args.pop();

    if !args.is_empty() {
        return Err(Box::new(ArgError::TooMany));
    }

    let offset = if let Some(offset) = parse_int(&offset) {
        offset as u64
    } else {
        return Err(Box::new(ArgError::Invalid));
    };

    let bp = if let Some(contract_id) = contract_id {
        if let Ok(contract_id) = contract_id.parse::<ContractId>() {
            fuel_vm::prelude::Breakpoint::new(contract_id, offset)
        } else {
            return Err(Box::new(ArgError::Invalid));
        }
    } else {
        fuel_vm::prelude::Breakpoint::script(offset)
    };

    state.client.set_breakpoint(bp).await?;

    Ok(())
}

async fn cmd_registers(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);

    if args.is_empty() {
        for r in 0..VM_REGISTER_COUNT {
            let value = state.client.read_register(r.try_into().unwrap()).await?;
            println!("reg[{:#x}] = {:<8} # {}", r, value, register_name(r));
        }
    } else {
        for arg in &args {
            if let Some(v) = parse_int(arg) {
                if v < VM_REGISTER_COUNT {
                    let value = state.client.read_register(v.try_into().unwrap()).await?;
                    println!("reg[{:#02x}] = {:<8} # {}", v, value, register_name(v));
                } else {
                    println!("Register index too large {}", v);
                    return Ok(());
                }
            } else if let Some(index) = names::register_index(arg) {
                let value = state
                    .client
                    .read_register(index.try_into().unwrap())
                    .await?;
                println!("reg[{:#02x}] = {:<8} # {}", index, value, arg);
            } else {
                println!("Unknown register name {}", arg);
                return Ok(());
            }
        }
    }

    Ok(())
}

async fn cmd_memory(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);

    let limit = args
        .pop()
        .map(|a| parse_int(&a).ok_or(ArgError::Invalid))
        .transpose()?
        .unwrap_or(WORD_SIZE * (VM_MAX_RAM as usize));

    let offset = args
        .pop()
        .map(|a| parse_int(&a).ok_or(ArgError::Invalid))
        .transpose()?
        .unwrap_or(0);

    if !args.is_empty() {
        return Err(Box::new(ArgError::TooMany));
    }

    let mem = state
        .client
        .read_memory(offset.try_into().unwrap(), limit.try_into().unwrap())
        .await?;

    for (i, chunk) in mem.chunks(WORD_SIZE).enumerate() {
        print!(" {:06x}:", offset + i * WORD_SIZE);
        for byte in chunk {
            print!(" {:02x}", byte);
        }
        println!();
    }

    Ok(())
}

fn parse_int(s: &str) -> Option<usize> {
    let (s, radix) = if let Some(stripped) = s.strip_prefix("0x") {
        (stripped, 16)
    } else {
        (s, 10)
    };

    let s = s.replace('_', "");

    usize::from_str_radix(&s, radix).ok()
}
