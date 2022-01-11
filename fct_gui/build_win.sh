#combine first argument with "pc-windows-gnu"

TARGET="$1-pc-windows-gnu"
if [ $2 == "release" ]; then
    TARGET="$1-pc-windows-gnu"
    cargo build --target $TARGET --release
elif [ $2 == "debug" ] || [ -z "$2" ]; then
    TARGET="$1-pc-windows-gnu"
    cargo build --target $TARGET
fi

