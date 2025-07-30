use regex::Regex; fn main() { let regex = Regex::new(r"^[a-zA-Z0-9._/\-]+$").unwrap(); println\!("{}", regex.is_match("/tmp/test_client_server_comm.sock")); }
