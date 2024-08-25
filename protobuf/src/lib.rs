
pub mod protocol {
    include!(concat!(env!("OUT_DIR"), "/game.rs"));
}
pub mod rpc {
   include!(concat!(env!("OUT_DIR"), "/rpc.gate.rs"));
}