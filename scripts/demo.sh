#!/bin/bash
r1cs_file=${1}
wtns_file=${2}

curl -X POST http://localhost:8000/register  -F 'key=demo' -F "r1cs=@${r1cs_file}"
hex_proof=$(curl -X POST http://localhost:8000/prove -F key=demo -F "witness=@${wtns_file}" | jq .hex_proof)
curl -X POST  http://localhost:8000/verify -F key=demo -F "hex_proof=${hex_proof}"