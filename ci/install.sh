set -euxo pipefail

main() {
    if [ $TARGET != rustfmt ]; then
        rustup target add $TARGET
    else
        rustup component add rustfmt
    fi
}

main
