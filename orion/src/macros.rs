#[macro_export]
macro_rules! register_rpc {
    ($($name:literal => $handler:expr),*) => {
        $crate::rpc::register_rpc_routes(vec![$($crate::rpc::Pair($name, $crate::rpc::rpc($handler))),*]);
    };
}
