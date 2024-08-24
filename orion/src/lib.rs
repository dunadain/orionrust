use std::fs::OpenOptions;
use std::io::Write;

mod app;
pub use app::appinfo;
pub use app::Application;
mod net;
pub use net::socket_handle::SocketHandle;
pub use net::tcp::serve_tcp;
pub use net::tcp::tcp_actors::TcpSocketHandle;
pub use net::tcp::SocketListener;
mod nats;
pub use nats::nats_client;
pub use nats::nats_msg;
pub use nats::rpc_subscriber;
pub mod async_redis;
pub mod macros;
pub mod rpc;

pub fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("panic.log")
            .unwrap();

        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            writeln!(file, "{} Panic occurred: {}", appinfo().server_type(), s).unwrap();
        } else {
            writeln!(file, "{} Panic occurred", appinfo().server_type()).unwrap();
        }

        if let Some(location) = panic_info.location() {
            writeln!(
                file,
                "Panic location: {}:{}",
                location.file(),
                location.line()
            )
            .unwrap();
        } else {
            writeln!(file, "Panic location: unknown").unwrap();
        }
        writeln!(file, "My backtrace: {:#?}", backtrace).unwrap();
    }));
}
