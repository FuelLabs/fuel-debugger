use clap::Parser;
use shellfish::async_fn;
use shellfish::{Command as ShCommand, Shell};
use std::error::Error;
use std::net::SocketAddr;
use std::{fmt, net};
use tokio::net::TcpListener;

use fuel_vm::consts::{VM_MAX_RAM, WORD_SIZE};
use fuel_vm::prelude::{Breakpoint, ContractId};

use fuel_debugger::{Command, Response};

use fuel_debugger_client::{names, Client, Listener};

#[derive(Parser, Debug)]
pub struct Opt {
    #[clap(long = "ip", default_value = "127.0.0.1", parse(try_from_str))]
    pub ip: net::IpAddr,

    #[clap(long = "port", default_value = "4001")]
    pub port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Opt::parse();

    // let mut rl = Editor::<()>::new();

    let listener = Listener::new(TcpListener::bind((config.ip, config.port)).await?);

    println!(
        "Listening for connections at {:?}",
        SocketAddr::from((config.ip, config.port))
    );

    let mut shell = Shell::new_async(State::default(), ">> ");

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

    command!(cmd_version, "query version information", ["v", "version"]);
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
        ["r", "registers"]
    );
    command!(cmd_memory, "[offset] limit -- dump memory", ["m", "memory"]);

    loop {
        let (client, addr) = listener.accept().await?;
        println!("Connected (remote {:?})", addr);

        shell.state.connection = Some(client);
        shell.run_async().await?;

        println!("Disconnected");
    }
}

#[derive(Default)]
struct State {
    connection: Option<Client>,
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

async fn cmd_version(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    if args.len() != 0 {
        return Err(Box::new(ArgError::TooMany));
    }
    println!(
        "{:?}",
        state
            .connection
            .as_mut()
            .unwrap()
            .cmd(&Command::Version)
            .await?
    );
    Ok(())
}

async fn cmd_continue(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    if args.len() != 0 {
        return Err(Box::new(ArgError::TooMany));
    }
    println!(
        "{:?}",
        state
            .connection
            .as_mut()
            .unwrap()
            .cmd(&Command::Continue)
            .await?
    );
    Ok(())
}

async fn cmd_step(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    if args.len() > 1 {
        return Err(Box::new(ArgError::TooMany));
    }
    println!(
        "{:?}",
        state
            .connection
            .as_mut()
            .unwrap()
            .cmd(&Command::SingleStepping(
                args.get(0)
                    .map(|v| ["off", "no", "disable"].contains(&v.as_str()))
                    .unwrap_or(false),
            ))
            .await?
    );
    Ok(())
}

async fn cmd_breakpoint(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);
    let offset = args.pop().ok_or(Box::new(ArgError::NotEnough))?;
    let contract_id = args.pop();

    if !args.is_empty() {
        return Err(Box::new(ArgError::TooMany));
    }

    let offset = if let Some(offset) = parse_int(&offset) {
        offset as u64
    } else {
        return Err(Box::new(ArgError::Invalid));
    };

    let b = if let Some(contract_id) = contract_id {
        if let Ok(contract_id) = contract_id.parse::<ContractId>() {
            Breakpoint::new(contract_id, offset)
        } else {
            return Err(Box::new(ArgError::Invalid));
        }
    } else {
        Breakpoint::script(offset)
    };

    println!(
        "{:?}",
        state
            .connection
            .as_mut()
            .unwrap()
            .cmd(&Command::Breakpoint(b))
            .await?
    );

    Ok(())
}

async fn cmd_registers(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);

    let regs = match state
        .connection
        .as_mut()
        .unwrap()
        .cmd(&Command::ReadRegisters)
        .await?
    {
        Response::ReadRegisters(regs) => regs,
        other => panic!("Unexpected response {:?}", other),
    };

    for arg in &args {
        if let Some(v) = parse_int(&arg) {
            if v < regs.len() {
                println!("{:?}", regs[v]);
            } else {
                println!("Register index too large {}", v);
                return Ok(());
            }
        } else if let Some(i) = names::REGISTERS.get(&arg) {
            println!("{:?}", regs[*i]);
        } else {
            println!("Unknown register name {}", arg);
            return Ok(());
        }
    }

    if args.is_empty() {
        println!("{:?}", regs);
    }

    Ok(())
}

async fn cmd_memory(state: &mut State, mut args: Vec<String>) -> Result<(), Box<dyn Error>> {
    args.remove(0);

    let limit = args
        .pop()
        .map(|a| parse_int(&a).ok_or(ArgError::Invalid))
        .transpose()?
        .unwrap_or(WORD_SIZE * VM_MAX_RAM as usize);

    let offset = args
        .pop()
        .map(|a| parse_int(&a).ok_or(ArgError::Invalid))
        .transpose()?
        .unwrap_or(0);

    if !args.is_empty() {
        return Err(Box::new(ArgError::TooMany));
    }

    let mem = match state
        .connection
        .as_mut()
        .unwrap()
        .cmd(&Command::ReadMemory {
            start: offset,
            len: limit,
        })
        .await?
    {
        Response::ReadMemory(mem) => mem,
        other => panic!("Unexpected response {:?}", other),
    };

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
