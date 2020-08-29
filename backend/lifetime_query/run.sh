#!/usr/bin/env bash

SHELL_FOLDER=$(dirname $(readlink -f "$0"))
rustup component add rust-src > /dev/null
rustup component add rustc-dev > /dev/null
if [ -z "$1" ]; then
	echo "No detecting directory is provided"
	exit 1
fi
cd $SHELL_FOLDER
cargo build --release
export RUSTC=${SHELL_FOLDER}/target/release/vrlifetime-backend
export RUSTC_FLAGS="-Zalways-encode-mir"
export RUST_BACKTRACE=full

cargo_dir_file=$(realpath cargo_dir.txt)
rm $cargo_dir_file
touch $cargo_dir_file

pushd "$1" > /dev/null
#cargo clean
cargo_tomls=$(find . -name "Cargo.toml")
for cargo_toml in ${cargo_tomls[@]}
do
#	echo $cargo_toml
	echo $(dirname $cargo_toml) >> $cargo_dir_file
done

IFS=$'\n' read -d '' -r -a lines < ${cargo_dir_file}
for cargo_dir in ${lines[@]}
do
	echo ${cargo_dir}
	pushd ${cargo_dir} > /dev/null
	cargo check 
	popd > /dev/null
done
popd > /dev/null

#pushd "$1" > /dev/null
#cargo clean
#cargo check
#popd > /dev/null
