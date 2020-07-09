#!/bin/bash
case "$1" in
    --model-only)
        args=()
        ;;
    -h|--help)
        echo "usage: $0 [--model-only] [-h|--help]"
        exit
        ;;
    '')
        args=("--features=native_instrs")
        ;;
    *)
        echo "invalid option; use --help for help" >&2
        exit 1
        ;;
esac
exec cargo run "${args[@]}" > "output-for-`git describe`.json"

