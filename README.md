# [GAME-NAME]

To run a quick dev build:
`cargo run -p shooter --features dev-local`
`cargo run -p shooter-server --features dev-local`

To fully build:  
`cargo build -p shooter-server --release`  
Put the server on the desktop, run it  
`.\build.ps1` on burvy.dev repo  

The released executables are at:
`C:\ct\burvy-game\x86_64-pc-windows-msvc\release\`

When the game is built with dev-local,  
The client has no TLS validation,  
The server connects to localhost and self signs without PEM files  
The protocol points towawrds localhost instead of my server IP

## Inputs
Remember that new player inputs must be added in `PlayerInputs` 
which is in `protocol.rs` in PROTOCOL  
The client gets to send input through `buffer_input` in `net.rs` 
on the CLIENT

## Components
Look at the basic `Interactable` component in `definition.rs` of `protocol/stuff`. Remember those traits, 
since we need those to replicate over the protocol.
