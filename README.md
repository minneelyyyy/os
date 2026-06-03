
This program requires the uefi target set up in your rust toolchain.

```
$ rustup target add x86_64-unknown-uefi`
```

You must install `nasm` to build this, and `qemu` and `OVMF` firmware to run it.

```
$ apt install nasm qemu OVMF
```

and then

```
$ cargo run
```
