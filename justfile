# Run code coverage
coverage:
    export CARGO_INCREMENTAL=0 \
    && export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Zno-landing-pads" \
    && cargo +nightly build \
    && cargo +nightly test
    ~/.cargo/bin/grcov ./target/debug/ \
        -s . \
        -t html \
        --llvm \
        --branch \
        --ignore-not-existing \
        -o ./target/debug/coverage/
    echo $PWD/target/debug/coverage/index.html