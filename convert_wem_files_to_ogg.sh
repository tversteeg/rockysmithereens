#!/bin/bash

export RUST_LOG=warn

FILES=$(cargo run --bin psarc_extract -- "$1" list | grep ".wem")

export RUST_LOG=trace

echo "$FILES" | while read line
do
	echo "$line"

	TARGET="${2:-/tmp}/${line##*/}"

	target/debug/psarc_extract "$1" extract "$line" "$TARGET"

	(cd ~/r/ww2ogg && ./ww2ogg --pcb packed_codebooks_aoTuV_603.bin --no-mod-packets "$TARGET")
done
