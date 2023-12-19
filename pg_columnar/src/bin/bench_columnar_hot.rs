fn main() {
    std::process::Command::new("sh")
        .arg("clickbench/paradedb/benchmark-hot.sh")
        .status()
        .expect("Failed to execute benchmark script");
}
