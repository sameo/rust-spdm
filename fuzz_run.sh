#!/bin/bash

pkill screen

if [ ! -d "fuzz-target/out" ];then
    mkdir fuzz-target/out
fi

for i in fuzz-target/out/*;do
    if [[ "`ls -A $i/default/crashes`" != "" ]];then
        echo -e "\033[31m There are some crashes \033[0m"
        echo -e "\033[31m Path in fuzz-target/out/$i/default/crashes \033[0m"
        exit
    fi
done

rm -rf fuzz-target/out/*
cmds=(
"rspversion"
"rspcapability"
"rspalgorithm"
"rspdigest"
"rspcertificate"
"rspchallenge"
"rspmeasurement"
"rspkeyexchange"
"rsppskexchange"
"finish_rsp"
"psk_finish_rsp"
"heartbeat_rsp"
"key_update_rsp"
"end_session_rsp"

"reqversion"
"reqcapability"
"reqalgorithm"
"reqdigest"
"reqcertificate"
"reqchallenge"
"reqmeasurement"
"key_exchange_req"
"psk_exchange_req"
"finish_req"
"psk_finish_req"
"heartbeat_req"
"key_update_req"
"end_session_req"
)

buildpackage=''
for i in ${cmds[@]};do
    buildpackage="-p $i $buildpackage";
done

echo "cargo afl build --features fuzz $buildpackage"

unset RUSTFLAGS
unset LLVM_PROFILE_FILE

if [[ $1 = "Scoverage" ]]; then
    echo "$1"
    export RUSTFLAGS="-Zinstrument-coverage"
    export LLVM_PROFILE_FILE='fuzz_run%m.profraw'
fi

if [[ $1 = "Gcoverage" ]]; then
    echo "$1"
    export CARGO_INCREMENTAL=0
    export RUSTDOCFLAGS="-Cpanic=abort"
    export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort"
fi

cargo afl build --features fuzz $buildpackage

for ((i=0;i<${#cmds[*]};i++))
do
    echo ${cmds[$i]}
    screen -ls | grep ${cmds[$i]}
    if [[ $? -ne 0 ]]
    then
    screen -dmS ${cmds[$i]}
    fi
    screen -x -S ${cmds[$i]} -p 0 -X stuff "cargo afl fuzz -i fuzz-target/in/${cmds[$i]} -o fuzz-target/out/${cmds[$i]} target/debug/${cmds[$i]}"
    screen -x -S ${cmds[$i]} -p 0 -X stuff $'\n'
    sleep 1800
    screen -S ${cmds[$i]} -X quit
    sleep 5
done

if [[ $1 = "Scoverage" || $1 = "Gcoverage" ]]; then
grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/fuzz_coverage/
fi
