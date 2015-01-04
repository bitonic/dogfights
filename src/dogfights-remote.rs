extern crate "dogfights-lib" as dogfights_lib;

fn main() {
    dogfights_lib::remote_client("127.0.0.1:10000", "127.0.0.1:10001")
}
