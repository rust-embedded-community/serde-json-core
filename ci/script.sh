set -euxo pipefail

main() {
    if [ $TARGET = x86_64-unknown-linux-gnu ]; then
        cargo check --target $TARGET
        cargo test --target $TARGET
        return
    fi

    xargo check --target $TARGET
}

if [ $TRAVIS_BRANCH != master ]; then
    main
fi
