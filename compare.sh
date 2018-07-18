#!/usr/bin/env bash
set -eux

rustc stream-rs.rs
./stream-rs < input.txt > output-rs.txt
stack stream-hs.hs < input.txt > output-hs.txt
exec diff output-rs.txt output-hs.txt
