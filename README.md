Program to analyze the behavior of Power ISA instructions

## Python Build Instructions

requires a recent version of Rust stable

```bash
cargo install maturin
git clone https://salsa.debian.org/Kazan-team/power-instruction-analyzer.git
cd power-instruction-analyzer
maturin build --cargo-extra-args=--features=python-extension
python3 -m pip install --user target/wheels/*.whl
```

alternatively, if in a python3 virtualenv:

```bash
git clone https://salsa.debian.org/Kazan-team/power-instruction-analyzer.git
cd power-instruction-analyzer
maturin develop --cargo-extra-args=--features=python-extension
```
or
```bash
git clone https://salsa.debian.org/Kazan-team/power-instruction-analyzer.git
cd power-instruction-analyzer
pip install .
```
