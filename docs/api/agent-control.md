# Agent control API

How a program drives a running game: an AI agent through MCP, a script through `industrial-era-ctl`, a test through the
control socket. Design and inventory: [agent-control.md](../agent-control.md); for players:
[playhorizon.dev/docs/agents](https://playhorizon.dev/docs/agents).

## Overview

The game listens on a local control socket (a Unix socket, or localhost TCP with a token on Windows), takes one
command per text line and answers one JSON line. `industrial-era-ctl` wraps it: it starts the game, sends commands and
serves the same commands as MCP tools. The launcher's MCP server offers them too, as `game_<tool>`.

| | Count |
| --- | --- |
| `industrial-era-ctl` commands | 96, in 8 groups (session, observe, move and look, act, screens and chat, multiplayer, world and cheats, scripting and benchmarks) |
| MCP tools | 88 (8 commands are CLI only) |
| Destructive commands (need `--confirm`) | 3 |
| Command names the game's socket answers | about 160, test hooks included (`industrial-era-ctl help --game`) |

## Guide

1. Build `industrial-era-ctl` (`cargo build --profile fast -p industrial-era-client-lib --bin industrial-era-ctl`), then
   `industrial-era-ctl --socket $PWD/ctl.sock launch --world W --creative --wait --resources <prepared files>`.
2. `industrial-era-ctl --socket … state`, `keys`, `turn`, `command`, `screenshot`, `quit`.
3. For an agent: `industrial-era-ctl mcp`, or the launcher's `industrial-era-launcher mcp` with `agent-config` to connect a client
   ([launcher-commands.md](../launcher-commands.md)).

## Reference

[industrial-era-ctl-commands.md](../industrial-era-ctl-commands.md), generated from the command table in
`crates/industrial-era-client/src/bin/industrial-era-ctl/commands.rs`: every command, its MCP tool and its arguments.

## Examples

- `tools/agent_control_e2e.py`: starts a game and drives it end to end through `industrial-era-ctl` and MCP.
- `tools/lan_e2e.py`: two games over LAN, both driven through their sockets.

## Stability

Preview ([stability.md](stability.md)). The commands in `industrial-era-ctl`'s table and their MCP tool names are what the
API offers; the other commands the game's socket answers are test hooks and experimental.

## Not there yet

- **A version that is checked.** `hello` answers it (and MCP `serverInfo` carries the release), but `industrial-era-ctl`
  still drives any game: it should refuse one whose line it doesn't speak (`industrial-era-ctl --api 0.1`).
- **Reply schemas.** Each command's reply described in the table (fields, types, units), so the reference documents
  replies as well as arguments and MCP tools declare an output schema.
- **Events as a stream.** `events --since` polls today; a subscription that pushes events as they happen.
- **Test hooks apart.** The hooks only harnesses use move under their own prefix, out of the agent's tool list.
- **Compatibility tests.** A transcript per protocol version: commands and the reply keys each must keep.

## Changelog

- 0.1.0: the first numbered line. `hello` answers `protocol`, `release` (the game's version, which MCP `serverInfo`
  carries too) and `features` (the command groups the game serves). Before it the command table and the generated
  reference were the contract.
