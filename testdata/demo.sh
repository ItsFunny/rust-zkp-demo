#!/bin/bash

# ./demo.sh demo ./circoms/mycircuit.r1cs ./circoms/witness.wtns
# ./demo.sh demo2 ./circoms/single_tx.r1cs ./circoms/single_tx.wtns

echorun() {
    echo "------> ["$@"]"
    $@
}
function failOnExit() {
    $@
   if [ $? -ne 0 ]; then
        echo "["$@"] failed"
        exit
    fi
}


key_name=${1}
r1cs_file=${2}
wtns_file=${3}

if  [ ! -n "$1" ] ;then
  echo "必须输入key name,如./demo.sh demo ./circoms/mycircuit.r1cs ./circoms/witness.wtns"
  exit
fi


if  [ ! -n "$2" ] ;then
  echo "必须输入r1cs_file,如./demo.sh demo ./circoms/mycircuit.r1cs ./circoms/witness.wtns"
  exit
fi

if  [ ! -n "$3" ] ;then
  echo "必须输入wtns_file,如./demo.sh demo ./circoms/mycircuit.r1cs ./circoms/witness.wtns"
  exit
fi



failOnExit curl -X POST http://localhost:8000/register  -F "key=${key_name}" -F "r1cs=@${r1cs_file}"
sleep 1
hex_proof=$(curl -X POST http://localhost:8000/prove -F "key=${key_name}" -F "witness=@${wtns_file}" | jq .hex_proof)
echo "============="
echo "============="
echo "hex proof 为:${hex_proof}"
echo "============="
echo "============="

sleep 2
verify=$(curl -X POST  http://localhost:8000/verify -F "key=${key_name}" -F "hex_proof=${hex_proof}"|jq .verify)

echo "校验结果为:${verify}"