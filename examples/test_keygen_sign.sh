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
    -a http://localhost:8080/ \ # address: sm manager address 
    -i $i \ # index: party index
    -n $n \ # number_of_parties: party threshold
    -t $t \ # threshold: party threshold
    -r group-8 \ # group: group name
    -o ./local-share$i.json) & # output keyfile path
    sleep 3
done

sleep 5
echo "sign"

for i in $(seq 1 $((t+1)));
do
    echo "signing for client $i out of $((t+1))"
     (../target/release/examples/gg20_signing \
    -a http://localhost:8080/ \ # address: sm manager address 
    -p 1,2 \ # parties: join party
    -d "Hello world!" \ # data: sign data
    -r group-8 \ # group: group name
    -l ./local-share$i.json) & # keyfile path
    sleep 3
done
