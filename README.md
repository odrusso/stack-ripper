
![Logo](https://raw.githubusercontent.com/odrusso/stack-ripper/main/resources/silkscreen_rx.svg)


# stack-ripper

The software to run on the 'stack-ripper' High Power Rocketry avionics platforms.

The board designs can be found [here](https://www.flux.ai/odrusso).


## Tech

### Hardware
To help contextualise the software decisions, here is a brief overview of the current hardware choices.

The reciever (`rx`) has:
- an ESP32-C3 MCU,
- an SX1278-based LoRA Radio

The transmitter (`tx`) additionally includes:
- a u-blox GNSS module,

### Software 
All written in async Rust, using [embassy](https://embassy.dev).

Each platform has a seperate binary, in `/src/bin/[platform].rs`.

Each prehipheral is maintained in a resuable library in `/src/[prehipheral].rs`


## Getting started

Install the prerequisites (not required if `rustup` automatically detects `rust-toolchain.toml`)

```bash
  rustup toolchain install stable --component rust-src
  rustup target add riscv32imc-unknown-none-elf
```

Install dependencies & build

```bash
  cargo build
```

Install flashing utility
```bash
  cargo install espflash
```

Flash a device (interactive) with the `rx` software
```bash
  cargo build --bin rx
```
