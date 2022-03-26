# Fuel Debugger

[![build](https://github.com/FuelLabs/fuel-debugger/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-debugger/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-debugger?label=latest)](https://crates.io/crates/fuel-debugger)
[![docs](https://docs.rs/fuel-debugger/badge.svg)](https://docs.rs/fuel-debugger/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Debugger attachable to FuelVM over a streaming message channel, such as a TCP socket. A CLI interface over TCP is provided as well.

## Technical details

Uses linefeed-delimited JSON messages. The fuel-core debug component performs the given commands sequentially and responds to each with a JSON reply. Each command results in exactly one reply.
