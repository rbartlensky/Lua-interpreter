#!/bin/sh

curl https://sh.rustup.rs -sSf | sh # rustc + cargo

sudo apt-get install autoconf \ # for multitime, lua, luajit
                     libreadline-dev \ # for lua, luajit
