#/bin/bash
# copy those cmd into shell 
cargo build --release --examples --no-default-features --features curv-kzen/num-bigint &&
../target/release/examples/gg20_sm_manager &
(sleep 1s;../target/release/examples/gg20_keygen \
    -a http://localhost:8080/ \
    -i 1 \
    -n 3 \
    -t 2 \
    -r group-8 \
    -o ./local-share1.json) &

(sleep 1s; ../target/release/examples/gg20_keygen \
    -a http://localhost:8080/ \
    -i 2 \
    -n 3 \
    -t 2 \
    -r group-8 \
    -o ./local-share2.json) &

(sleep 1s; ../target/release/examples/gg20_keygen \
    -a http://localhost:8080/ \
    -i 3 \
    -n 3 \
    -t 2 \
    -r group-8 \
    -o ./local-share3.json)