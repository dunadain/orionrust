use orion::register_rpc;
use protobuf::rpc::{HelloReply, HelloRequest};

pub fn register_rpc() {
    register_rpc!(
        "Greeter.SayHello" => test
    );
}

async fn test(req: HelloRequest) -> HelloReply {
    HelloReply {
        message: format!("Hello, {}!", req.name),
    }
}
