#/bin/bash
# copy those cmd into shell 
cargo build --release --examples --no-default-features --features curv-kzen/num-bigint

file_as_string=`cat test_params.json`

n=`echo "$file_as_string" | cut -d "\"" -f 4 `
t=`echo "$file_as_string" | cut -d "\"" -f 8 `

echo "Multi-party ECDSA parties:$n threshold:$t"


sleep 1


rm local-share?.json

killall gg20_sm_manager gg20_keygen_client gg20_sign_client 2> /dev/null

../target/release/examples/gg20_sm_manager &

sleep 2
echo "keygen part"

for i in $(seq 1 $n)
do
    echo "key gen for client $i out of $n"
    (../target/release/examples/gg20_keygen \
    -a http://localhost:8080/ \
    -i $i \
    -n $n \
    -t $t \
    -r group-8 \
    -o ./local-share$i.json) &
    sleep 3
done

sleep 5
echo "sign"

for i in $(seq 1 $((t+1)));
do
    echo "signing for client $i out of $((t+1))"
     (../target/release/examples/gg20_signing \
    -a http://localhost:8080/ \
    -p 1,2 \
    -d "Hello world!" \
    -r group-8 \
    -l ./local-share$i.json) &
    sleep 3
done

