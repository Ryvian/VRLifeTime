test_file=".installed"
rust_version="nightly-2020-05-10"
if [[ ! -f "$test_file" ]]; then
    rustup toolchain install $rust_version
    rustup "+$rust_version" component add rust-src &&
    rustup "+$rust_version" component add rustc-dev &&
    cd backend/lifetime_query &&
    cargo "+$rust_version" build --release &&
    cd ../rust-lock-bug-detector &&
    cargo "+$rust_version" install --path . &&
    cd ../.. &&
    touch $test_file
fi
    
