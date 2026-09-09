# [GAME-NAME]

To run a quick dev build:
`cargo run --features dev-insecure`

## Inputs
Remember that new player inputs must be added in `PlayerInputs` 
which is in `protocol.rs` in PROTOCOL  
The client gets to send input through `buffer_input` in `net.rs` 
on the CLIENT
