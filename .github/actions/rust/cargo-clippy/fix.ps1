# This file is automatically synchronized from https://github.com/Glatzel/template

$clippy_args = $(yq '.inputs.args.default' "$PSScriptRoot/action.yml") -split ' '
cargo clippy --fix $args -- $clippy_args
