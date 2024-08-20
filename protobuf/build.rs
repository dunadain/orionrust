fn main() {
    prost_build::compile_protos(&["src/greeter.proto"], &["src/"]).unwrap();
}
