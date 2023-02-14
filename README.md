# requirements

- rust
- circom

# prepare

cd testdata && ./prepare.sh

# start rust web server

cd testdata && ./start.sh

# test prove and verify

cd testdata && ./demo.sh demo ./circoms/mycircuit.r1cs ./circoms/witness.wtns