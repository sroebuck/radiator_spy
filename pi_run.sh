#! /bin/bash
PI=pi
# cross build --release --target=armv7-unknown-linux-gnueabihf && \
# scp target/armv7-unknown-linux-gnueabihf/release/radiator_spy pi: && \

rsync -av --delete -e ssh --exclude target --exclude ../cc1101/target --exclude .git --exclude ../cc1101/.git ../radiator_spy ../cc1101 $PI: && \
ssh $PI "cd radiator_spy && ~/.cargo/bin/cargo run"

# # Run radiator_spy on pi and kill when I CTRL-C locally...
# ssh -t -t pi "bash -O huponexit -c 'target/debug/radiator_spy'"
