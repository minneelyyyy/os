
This program requires the uefi target set up in your rust toolchain.

```
$ rustup target add x86_64-unknown-uefi`
```

You must install `qemu` and `OVMF` firmware to run it.

```
$ apt install qemu ovmf
```

and then

```
$ cargo run
```
