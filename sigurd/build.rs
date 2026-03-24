fn main() {
    include_packed::Config::new("drivers")
        .level(10)
        .build()
        .expect("Failed to pack drivers");
}