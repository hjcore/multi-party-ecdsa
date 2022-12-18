#/bin/bash
# copy those cmd into shell 
cargo build --release --examples --no-default-features --features curv-kzen/num-bigint

file_as_string=`cat test_params.json`

n=`echo "$file_as_string" | cut -d "\"" -f 4 `
t=`echo "$file_as_string" | cut -d "\"" -f 8 `
remove_index=4

echo "Multi-party ECDSA parties:$n threshold:$t"

sleep 1

rm local-share*

killall  gg20_keygen_client gg20_sign_client 2> /dev/null

../target/release/examples/gg20_sm_manager &

sleep 2
echo "keygen part"

<<ps
--address // sm mananger address
--index // party index
--number-of-parties // threshold total party.
--threshold  // threshold
--room      // MPC group
--output // keygen keypair output path
ps

for i in $(seq 1 $n)
do
    echo "key gen for client $i out of $n"
    (../target/release/examples/gg20_keygen \
    --address http://localhost:8080/ \
    --index $i \
    --number-of-parties $n \
    --threshold $t \
    --room group \
    --output ./local-share$i.json) & 
    sleep 5
done

<<ps
-a // sm mananger address
-p // join parties index
-d // sign data
-r // MPC group
-l // keyfile path
ps

sleep 5
echo "sign"
for i in $(seq 1 $((t+1)));
do
    echo "signing for client $i out of $((t+1))"
     (../target/release/examples/gg20_signing \
    -a http://localhost:8080/ \
    -p 1,2,3 \
    -d "Hello world!" \
    -r group-8 \
    -l ./local-share$i.json) &
    sleep 5
done

<<ps
--address //sm mananger address
--index // party index
--local-share // keyfile path
--number-of-parties // threshold total party.
--room      // MPC group
--output // dkr keypair output path
ps


for i in $(seq 1 $n)
do
    
    echo "dkr for client $i out of $((t+1))"

        (../target/release/examples/dkr \
        --address http://localhost:8080/ \
        --index $i \
        --local-share ./local-share$i.json \
        --number-of-parties $n \
        --room group-8 \
        --output ./local-share$i-dkr.json) &
    sleep 5
done

sleep 5

<<ps
-a // sm mananger address
-p // join parties index
-d // sign data
-r // MPC group
-l // keyfile path
--sign-index // keypair sign index
ps

echo "sign with dkr......"
for i in $(seq 1 3);
do
    echo "signing for client $i out of $((3)) sign-index"
     (../target/release/examples/gg20_signing \
    -a http://localhost:8080/ \
    -p 1,2,3 \
    -d "Hello world!!!" \
    -r group1 \
    --sign-index $((i)) \
    -l ./local-share$i-dkr.json ) &
    sleep 5
done