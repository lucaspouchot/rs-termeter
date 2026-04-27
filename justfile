build:
    cargo build --release
    @echo "Build completed successfully. You can find the executable in the target/release directory."

build-static:
    rustup target add x86_64-unknown-linux-musl
    cargo build --release --target x86_64-unknown-linux-musl
    @echo "Static build completed. Binary at target/x86_64-unknown-linux-musl/release/rs-termeter"

example: build
    target/release/rs-termeter test/example.txt

example-multiline: build
    target/release/rs-termeter test/example_multiline.txt -n "latency,throughput,success_rate" -p "5,25,75,95"

example-dual: build
    target/release/rs-termeter test/example_dual.txt -n "mo/s,rows/s" -p "5,95" --dual

example-pie: build
    target/release/rs-termeter test/example_pie.txt -n "Time per worker" --pie